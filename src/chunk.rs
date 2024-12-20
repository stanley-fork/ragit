use chrono::offset::Local;
use crate::ApiConfig;
use crate::error::Error;
use crate::index::{BuildConfig, Index, tfidf};
use crate::index::file::{AtomicToken, Image};
use crate::uid::Uid;
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use ragit_api::{
    ChatRequest,
    RecordAt,
};
use ragit_fs::{
    WriteMode,
    create_dir_all,
    exists,
    normalize,
    parent,
    read_bytes,
    set_extension,
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
mod renderable;
mod source;

pub use build_info::ChunkBuildInfo;
pub use renderable::RenderableChunk;
pub use source::ChunkSource;

pub const CHUNK_DIR_NAME: &str = "chunks";

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
        Some(_) => Err(Error::CorruptedFile(path.to_string())),
        None => {
            // simple hack: it throws the exact error that I want
            serde_json::from_slice::<Chunk>(&[])?;
            unreachable!()
        },
    }
}

/// It also creates a tfidf index of the chunk.
pub fn save_to_file(
    path: &str,
    chunk: &Chunk,

    // if the result json is bigger than threshold (in bytes), the file is compressed
    compression_threshold: u64,
    compression_level: u32,
    root_dir: &str,
) -> Result<(), Error> {
    let mut result = serde_json::to_vec_pretty(chunk)?;
    let tfidf_path = set_extension(path, "tfidf")?;
    let parent_path = parent(path)?;

    if !exists(&parent_path) {
        create_dir_all(&parent_path)?;
    }

    tfidf::save_to_file(
        &tfidf_path,
        chunk,
        root_dir,
    )?;

    if result.len() as u64 > compression_threshold {
        let mut compressed = vec![];
        let mut gz = GzEncoder::new(&result[..], Compression::new(compression_level));
        gz.read_to_end(&mut compressed)?;
        result = compressed;

        write_bytes(
            path,
            &[COMPRESS_PREFIX],
            WriteMode::CreateOrTruncate,
        )?;
    }

    else {
        write_bytes(
            path,
            &[UNCOMPRESS_PREFIX],
            WriteMode::CreateOrTruncate,
        )?;
    }

    Ok(write_bytes(
        path,
        &result,
        WriteMode::AlwaysAppend,
    )?)
}

impl Chunk {
    pub(crate) async fn create_chunk_from(
        tokens: &[AtomicToken],
        config: &BuildConfig,
        file: String,
        file_index: usize,
        api_config: &ApiConfig,
        pdl: &str,
        build_info: ChunkBuildInfo,
        previous_summary: Option<String>,
    ) -> Result<Self, Error> {
        let mut context = tera::Context::new();
        let mut chunk = vec![];
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
            }
        }

        context.insert("chunk", &chunk.concat());
        context.insert("max_summary_len", &config.max_summary_len);

        // It's ridiculous to ask for 300 characters from a 10 characters chunk.
        context.insert(
            "min_summary_len",
            &config.min_summary_len.min(approx_data_len / 2),
        );

        if let Some(previous_summary) = &previous_summary {
            context.insert("previous_summary", &escape_pdl_tokens(previous_summary));
        }

        let Pdl { messages, schema } = parse_pdl(
            pdl,
            &context,
            true,
            true,
        )?;
        let mut data = vec![];
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
            }
        }

        let data = data.concat();
        let request = ChatRequest {
            api_key: api_config.api_key.clone(),
            messages,
            model: api_config.model,
            frequency_penalty: None,
            max_tokens: None,
            max_retry: api_config.max_retry,
            sleep_between_retries: api_config.sleep_between_retries,
            timeout: api_config.timeout,
            temperature: None,
            record_api_usage_at: api_config.dump_api_usage_at.clone().map(
                |path| RecordAt { path, id: String::from("create_chunk_from") }
            ),
            dump_pdl_at: api_config.create_pdl_path("create_chunk_from"),
            dump_json_at: api_config.dump_log_at.clone(),
            schema,
            schema_max_try: 3,
        };
        let response = request.send_and_validate::<ChunkSchema>(ChunkSchema::dummy(&data, config.max_summary_len)).await?;
        let mut result = Chunk {
            data,
            images,
            char_len,
            image_count,
            title: response.title,
            summary: response.summary,
            source: ChunkSource::File { path: normalize(&file)?, index: file_index },
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
        match &self.source {
            ChunkSource::File { path, .. } => path.to_string(),
            ChunkSource::Chunks(_) => todo!(),
        }
    }
}

pub fn merge_and_convert_chunks(index: &Index, chunks: Vec<Chunk>) -> Result<Vec<RenderableChunk>, Error> {
    let mut merge_candidates = HashSet::new();
    let mut curr_chunks = HashMap::new();

    for chunk in chunks.into_iter() {
        match &chunk.source {
            ChunkSource::File { path, index } => {
                merge_candidates.insert((path.clone(), *index + 1));
                assert!(curr_chunks.insert((path.clone(), *index), chunk).is_none());
            },
            ChunkSource::Chunks(_) => {},  // it's unsearchable
        }
    }

    // it has to merge from left to right
    let mut merge_candidates: Vec<_> = merge_candidates.into_iter().collect();
    merge_candidates.sort_by_key(|(_, index)| *index);

    for candidate in merge_candidates.iter() {
        if curr_chunks.contains_key(candidate) {
            let pre = curr_chunks.remove(&(candidate.0.clone(), candidate.1 - 1)).unwrap();
            let post = curr_chunks.remove(candidate).unwrap();
            curr_chunks.insert((candidate.0.clone(), candidate.1), merge_chunks(pre, post));

            return merge_and_convert_chunks(index, curr_chunks.into_values().collect());
        }
    }

    let mut result = Vec::with_capacity(curr_chunks.len());

    for (_, chunk) in curr_chunks.into_iter() {
        result.push(chunk.into_renderable(index)?);
    }

    Ok(result)
}

fn merge_chunks(pre: Chunk, post: Chunk) -> Chunk {
    let ChunkSource::File { path: pre_path, index: pre_index } = pre.source.clone() else { unreachable!() };
    let ChunkSource::File { path: post_path, index: post_index } = post.source.clone() else { unreachable!() };
    assert_eq!(pre_path, post_path);
    assert_eq!(pre_index + 1, post_index);
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
        source: ChunkSource::File { path: post_path, index: post_index },
        timestamp: Local::now().timestamp(),

        // if source is `File`, it must be searchable
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

#[derive(Deserialize)]
struct ChunkSchema {
    title: String,
    summary: String,
}

impl ChunkSchema {
    pub fn dummy(data: &str, len: usize) -> Self {
        ChunkSchema {
            title: String::from("untitled"),
            summary: data.chars().take(len).collect(),
        }
    }
}
