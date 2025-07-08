use chrono::Local;
use crate::error::Error;
use crate::index::{Index, tfidf};
use crate::index::file::{AtomicToken, Image};
use crate::uid::Uid;
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use ragit_api::Request;
use ragit_fs::{
    WriteMode,
    exists,
    normalize,
    parent,
    read_bytes,
    set_extension,
    try_create_dir,
    write_bytes,
};
use ragit_pdl::{
    Pdl,
    encode_base64,
    escape_pdl_tokens,
    parse_pdl,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;

mod build_info;
mod multi_modal;
mod render;
mod source;

#[cfg(test)]
mod tests;

pub use build_info::ChunkBuildInfo;
pub use multi_modal::{MultiModalContent, into_multi_modal_contents};
pub use render::RenderedChunk;
pub use source::ChunkSource;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Chunk {
    pub data: String,
    pub images: Vec<Uid>,
    pub char_len: usize,

    /// It's not always `images.len()`. If the same image appears twice,
    /// it's pushed only once to `images` but twice to `image_count`
    pub image_count: usize,

    pub title: String,
    pub summary: String,

    pub source: ChunkSource,
    pub uid: Uid,
    pub build_info: ChunkBuildInfo,
    pub timestamp: i64,

    /// Chunks built from `rag build` are always searchable.
    /// Some chunks (e.g. summary of the entire knowledge-base) have no
    /// `data` and must be excluded in default RAG pipeline.
    pub searchable: bool,
}

const COMPRESS_PREFIX: u8 = b'c';
const UNCOMPRESS_PREFIX: u8 = b'\n';  // I want it to be compatible with json syntax highlighters

pub fn load_from_file(path: &str) -> Result<Chunk, Error> {
    let content = read_bytes(path)?;

    match content.get(0) {
        Some(b) if *b == COMPRESS_PREFIX => {
            let mut decompressed = vec![];
            let mut gz = GzDecoder::new(&content[1..]);
            gz.read_to_end(&mut decompressed)?;

            Ok(serde_json::from_slice::<Chunk>(&decompressed)?)
        },
        Some(b) if *b == UNCOMPRESS_PREFIX => Ok(serde_json::from_slice::<Chunk>(&content[1..])?),
        Some(b) => Err(Error::CorruptedFile { path: path.to_string(), message: Some(format!("unexpected chunk prefix: `{b}`")) }),
        None => {
            // simple hack: it throws the exact error that I want
            serde_json::from_slice::<Chunk>(&[])?;
            unreachable!()
        },
    }
}

pub fn save_to_file(
    path: &str,
    chunk: &Chunk,

    // if the result json is bigger than threshold (in bytes), the file is compressed
    compression_threshold: u64,
    compression_level: u32,
    root_dir: &str,
    create_tfidf: bool,
) -> Result<(), Error> {
    let mut result = serde_json::to_vec_pretty(chunk)?;
    let tfidf_path = set_extension(path, "tfidf")?;
    let parent_path = parent(path)?;

    if !exists(&parent_path) {
        try_create_dir(&parent_path)?;
    }

    if create_tfidf {
        tfidf::save_to_file(
            &tfidf_path,
            chunk,
            root_dir,
        )?;
    }

    let mut bytes = if result.len() as u64 > compression_threshold {
        let mut compressed = vec![];
        let mut gz = GzEncoder::new(&result[..], Compression::new(compression_level));
        gz.read_to_end(&mut compressed)?;
        result = compressed;

        vec![COMPRESS_PREFIX]
    }

    else {
        vec![UNCOMPRESS_PREFIX]
    };

    bytes.append(&mut result);
    Ok(write_bytes(
        path,
        &bytes,
        WriteMode::Atomic,
    )?)
}

impl Chunk {
    pub fn dummy(data: String, source: ChunkSource) -> Self {
        let mut result = Chunk {
            images: vec![],
            char_len: data.chars().count(),
            image_count: 0,
            title: String::new(),
            summary: String::new(),
            uid: Uid::dummy(),
            timestamp: Local::now().timestamp(),
            searchable: true,
            build_info: ChunkBuildInfo::dummy(),
            data,
            source,
        };

        result.uid = Uid::new_chunk(&result);
        result
    }

    pub(crate) async fn create_chunk_from(
        index: &Index,
        tokens: &[AtomicToken],
        file: String,
        file_index: usize,
        build_info: ChunkBuildInfo,
        previous_turn: Option<(Chunk, ChunkSchema)>,
        extra_info: Option<ChunkExtraInfo>,
    ) -> Result<Self, Error> {
        let mut context = tera::Context::new();
        let mut chunk = vec![];  // what LLM actually sees when building a chunk
        let mut approx_data_len = 0;

        for token in tokens.iter() {
            match token {
                AtomicToken::String { data, .. } => {
                    approx_data_len += data.chars().count();
                    chunk.push(escape_pdl_tokens(data));
                },
                AtomicToken::Image(Image { bytes, image_type, .. }) => {
                    approx_data_len += 10;
                    chunk.push(format!("<|raw_media({}:{})|>", image_type.to_extension(), encode_base64(&bytes)));
                },

                // If this branch is reached, that means `FileReader::generate_chunk` has
                // failed to fetch the image from web.
                AtomicToken::WebImage { subst, url: _ } => {
                    approx_data_len += subst.chars().count();
                    chunk.push(subst.clone());
                },

                // These tokens are supposed to be filtered out before passed to this function
                AtomicToken::PageBreak
               | AtomicToken::ChunkExtraInfo(_) => {
                    // invisible
                },
            }
        }

        context.insert("chunk", &chunk.concat());
        context.insert("max_summary_len", &index.build_config.max_summary_len);

        // It's ridiculous to ask for a 300 characters summary from a 10 characters chunk.
        context.insert(
            "min_summary_len",
            &index.build_config.min_summary_len.min(approx_data_len / 2),
        );

        if let Some((previous_chunk, previous_schema)) = &previous_turn {
            let previous_request = previous_chunk.clone().render(index)?;
            context.insert("previous_request", &previous_request.pdl_data);
            context.insert("previous_response", &previous_schema.render());
        }

        let Pdl { messages, schema } = parse_pdl(
            &index.get_prompt("summarize")?,
            &context,
            "/",  // TODO: `<|media|>` is not supported for this prompt
            true,
        )?;
        let mut data = vec![];  // data that's actually saved to the chunk file
        let mut images = vec![];
        let mut char_len = 0;
        let mut image_count = 0;

        for r in tokens.iter() {
            match r {
                AtomicToken::String { data: s, char_len: n } => {
                    data.push(s.clone());
                    char_len += *n;
                },
                AtomicToken::Image(i) => {
                    images.push(i.uid);
                    image_count += 1;
                    data.push(format!("img_{}", i.uid));
                },

                // If this branch is reached, that means `FileReader::generate_chunk` has
                // failed to fetch the image from web.
                AtomicToken::WebImage { subst, .. } => {
                    data.push(subst.clone());
                },

                // invisible
                // it must be filtered out before passed to this function
                AtomicToken::PageBreak
               | AtomicToken::ChunkExtraInfo(_) => {},
            }
        }

        let data = data.concat();
        let request = Request {
            messages,
            model: index.get_model_by_name(&index.api_config.model)?,
            max_retry: index.api_config.max_retry,
            sleep_between_retries: index.api_config.sleep_between_retries,
            timeout: index.api_config.timeout,
            dump_api_usage_at: index.api_config.dump_api_usage_at(&index.root_dir, "create_chunk_from"),
            dump_pdl_at: index.api_config.create_pdl_path(&index.root_dir, "create_chunk_from"),
            dump_json_at: index.api_config.dump_log_at(&index.root_dir),
            schema,
            schema_max_try: 3,
            ..Request::default()
        };

        // some apis reject empty requests
        let response = if data.is_empty() {
            ChunkSchema::empty()
        } else {
            request.send_and_validate::<ChunkSchema>(ChunkSchema::dummy(&data, index.build_config.max_summary_len)).await?
        };

        let mut result = Chunk {
            data,
            images,
            char_len,
            image_count,
            title: response.title,
            summary: response.summary,
            source: ChunkSource::File {
                path: normalize(&file)?,
                index: file_index,
                page: extra_info.map(|i| i.page_no).unwrap_or(None),
            },
            searchable: true,
            uid: Uid::dummy(),
            build_info,
            timestamp: Local::now().timestamp(),
        };
        let chunk_uid = Uid::new_chunk(&result);
        result.uid = chunk_uid;
        Ok(result)
    }

    pub fn render_source(&self) -> String {
        self.source.render()
    }

    pub fn sortable_string(&self) -> String {
        self.source.sortable_string()
    }

    pub(crate) fn get_approx_size(&self) -> usize {  // in bytes
        self.data.len() + self.title.len() + self.summary.len()
    }
}

/// It does some preprocessing on chunks, before fed to LLMs.
///
/// 1. If there are multiple chunks from the same file, it sorts the chunks.
/// 2. If there are consecutive chunks, it merges them. It handles sliding windows.
pub fn merge_and_convert_chunks(index: &Index, chunks: Vec<Chunk>) -> Result<Vec<RenderedChunk>, Error> {
    let mut merge_candidates = HashSet::new();
    let mut curr_chunks = HashMap::new();
    let mut unmergeable_chunks = vec![];

    for chunk in chunks.into_iter() {
        match &chunk.source {
            ChunkSource::File { path, index, page: _ } if *index > 0 => {
                merge_candidates.insert((path.clone(), *index - 1));
                curr_chunks.insert((path.clone(), *index), chunk);
            },
            ChunkSource::File { path, index, page: _ } => {
                curr_chunks.insert((path.clone(), *index), chunk);
            },
            // NOTE: there used to be another `ChunkSource`
            _ => { unmergeable_chunks.push(chunk); },
        }
    }

    // it has to merge from right to left
    let mut merge_candidates: Vec<_> = merge_candidates.into_iter().collect();
    merge_candidates.sort_by_key(|(_, index)| usize::MAX - *index);

    for candidate in merge_candidates.iter() {
        if curr_chunks.contains_key(candidate) {
            let pre = curr_chunks.remove(candidate).unwrap();
            let post = curr_chunks.remove(&(candidate.0.clone(), candidate.1 + 1)).unwrap();
            curr_chunks.insert((candidate.0.clone(), candidate.1), merge_chunks(pre, post));
            let curr_chunks_vec = vec![
                curr_chunks.into_values().collect(),
                unmergeable_chunks,
            ].concat();

            return merge_and_convert_chunks(index, curr_chunks_vec);
        }
    }

    // Chunks are sorted by file name and index
    // 1. sort by index: It makes more sense to preserve the order
    // 2. sort by file name: In order to run tests, the order has to be deterministic.
    let mut curr_chunks = curr_chunks.into_values().collect::<Vec<_>>();
    curr_chunks.sort_by_key(
        |chunk| match &chunk.source {
            ChunkSource::File { index, .. } => *index,
        }
    );
    curr_chunks.sort_by_key(
        |chunk| match &chunk.source {
            ChunkSource::File { path, .. } => path.to_string(),
        }
    );

    let mut result = Vec::with_capacity(curr_chunks.len());

    for chunk in unmergeable_chunks.into_iter() {
        result.push(chunk.render(index)?);
    }

    for chunk in curr_chunks.into_iter() {
        result.push(chunk.render(index)?);
    }

    Ok(result)
}

fn merge_chunks(pre: Chunk, post: Chunk) -> Chunk {
    let ChunkSource::File {
        path: pre_path,
        index: pre_index,
        page: pre_page,
    } = pre.source.clone() else { unreachable!() };
    let ChunkSource::File {
        path: post_path,
        index: post_index,
        page: post_page,
    } = post.source.clone() else { unreachable!() };
    assert_eq!(pre_path, post_path);
    assert_eq!(pre_index + 1, post_index);
    let page_no = if pre_page == post_page {
        pre_page
    } else {
        None
    };

    let Chunk {
        data: data_pre,
        images: images_pre,
        ..
    } = pre;
    let Chunk {
        data: data_post,
        images: images_post,
        ..
    } = post;

    let new_data = merge_overlapping_strings(data_pre.as_bytes(), data_post.as_bytes());

    // dedup
    let new_images = images_pre.into_iter().chain(images_post.into_iter()).collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();

    Chunk {
        char_len: new_data.chars().count(),
        data: new_data,
        images: new_images,
        image_count: 0,  // TODO: count images
        timestamp: Local::now().timestamp(),

        // When 1st and 2nd chunks are merged, the result is 1st, not 2nd.
        source: ChunkSource::File { path: pre_path, index: pre_index, page: page_no },

        // If source is `File`, it must be searchable
        searchable: true,

        // TODO: is it okay to leave these fields empty?
        summary: String::new(),
        title: String::new(),
        uid: Uid::dummy(),
        build_info: ChunkBuildInfo::dummy(),
    }
}

fn merge_overlapping_strings(s1: &[u8], s2: &[u8]) -> String {
    let min_len = s1.len().min(s2.len());
    let mut index = 0;

    for i in (0..=min_len).rev() {
        if s1.ends_with(&s2[..i]) {
            index = i;
            break;
        }
    }

    format!("{}{}", String::from_utf8_lossy(s1), String::from_utf8_lossy(&s2[index..]).to_string())
}

#[derive(Clone, Deserialize)]
pub struct ChunkSchema {
    pub title: String,
    pub summary: String,
}

impl ChunkSchema {
    pub fn dummy(data: &str, len: usize) -> Self {
        ChunkSchema {
            title: String::from("untitled"),
            summary: data.chars().take(len).collect::<String>().replace("\n", " "),
        }
    }

    pub fn empty() -> Self {
        ChunkSchema {
            title: String::from("an empty chunk"),
            summary: String::from("this is an empty chunk"),
        }
    }

    pub fn render(&self) -> String {
        format!(
"{}
    \"title\": {:?},
    \"summary\": {:?}
{}",
            '{',
            self.title,
            self.summary,
            '}',
        )
    }
}

impl From<&Chunk> for ChunkSchema {
    fn from(chunk: &Chunk) -> ChunkSchema {
        ChunkSchema {
            title: chunk.title.clone(),
            summary: chunk.summary.clone(),
        }
    }
}

/// I know it's becoming overly complicated... but I cannot help it.
/// Sometimes `FileReader`s want to add extra information to a chunk
/// (e.g. page number) that is only available to them. In those cases,
/// they can use `AtomicToken::PageBreak { extra_info: ChunkExtraInfo }`
/// to pass the information.
#[derive(Clone, Copy, Debug)]
pub struct ChunkExtraInfo {
    pub page_no: Option<usize>,
}

impl ChunkExtraInfo {
    /// Sometimes a file reader might generate multiple `ChunkExtraInfo`s
    /// for a single chunk. In such cases, those extra infos have to be merged.
    pub fn merge(&self, other: &ChunkExtraInfo) -> ChunkExtraInfo {
        let page_no = match (self.page_no, other.page_no) {
            (Some(p), Some(_)) => Some(p),  // there's nothing we can do
            (Some(p), None) | (None, Some(p)) => Some(p),
            (None, None) => None,
        };

        ChunkExtraInfo { page_no }
    }
}
