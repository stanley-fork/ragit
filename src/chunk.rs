use crate::ApiConfig;
use crate::error::Error;
use crate::index::{Config as BuildConfig, Index, file::AtomicToken, tfidf};
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use json::JsonValue;
use ragit_api::{
    ChatRequest,
    Message,
    MessageContent,
    RecordAt,
    Role,
    messages_from_pdl,
};
use ragit_fs::{
    WriteMode,
    join,
    normalize,
    read_bytes,
    set_ext,
    write_bytes,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::collections::{HashMap, HashSet};
use std::io::Read;

mod build_info;
mod renderable;

pub use build_info::BuildInfo;
pub use renderable::RenderableChunk;

// I wanted it to be u128, but serde_json does not support u128
pub type Uid = String;
pub const CHUNK_DIR_NAME: &str = "chunks";
pub const CHUNK_INDEX_DIR_NAME: &str = "chunk_index";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Chunk {
    pub data: String,

    // it's both key and path of an image
    // for ex, if "abcdef" is in `images`,
    // replace "img_abcdef" in `data` with `.rag_index/images/abcdef.png`
    pub images: Vec<String>,
    pub char_len: usize,

    // it's not always `images.len()`
    // if the same image appears twice, it's pushed only once to `images` but twice to `image_count`
    pub image_count: usize,

    pub title: String,
    pub summary: String,

    pub file: String,  // rel path
    pub index: usize,  // index in the file

    // unique identifier for chunks
    pub uid: Uid,
    pub build_info: BuildInfo,

    // if it belongs to an external base, the name of the
    // base is kept here
    #[serde(skip)]
    pub external_base: Option<String>,
}

const COMPRESS_PREFIX: u8 = b'c';
const UNCOMPRESS_PREFIX: u8 = b'\n';  // I want it to be compatible with json syntax highlighters

pub fn load_from_file(path: &str) -> Result<Vec<Chunk>, Error> {
    let content = read_bytes(path)?;

    match content.get(0) {
        Some(b) if *b == COMPRESS_PREFIX => {
            let mut decompressed = vec![];
            let mut gz = GzDecoder::new(&content[1..]);
            gz.read_to_end(&mut decompressed)?;

            Ok(serde_json::from_slice::<Vec<Chunk>>(&decompressed)?)
        },
        Some(b) if *b == UNCOMPRESS_PREFIX => Ok(serde_json::from_slice::<Vec<Chunk>>(&content[1..])?),
        Some(b) => {
            Err(Error::InvalidChunkPrefix(*b))
        },
        None => {
            // simple hack: it throws the exact error that I want
            serde_json::from_slice::<Vec<Chunk>>(&[])?;
            unreachable!()
        },
    }
}

pub fn save_to_file(
    path: &str,
    chunks: &[Chunk],

    // if the result json is bigger than threshold (in bytes), the file is compressed
    compression_threshold: u64,
    compression_level: u32,
) -> Result<(), Error> {
    let mut result = serde_json::to_vec_pretty(chunks)?;
    let tfidf_path = set_ext(path, "tfidf")?;
    tfidf::save_to_file(
        &tfidf_path,
        chunks,
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
    pub fn render_source(&self) -> String {
        if let Some(external_base) = &self.external_base {
            join(
                external_base,
                &self.file,
            ).unwrap()
        }

        else {
            self.file.to_string()
        }
    }

    pub async fn create_chunk_from(
        tokens: &[AtomicToken],
        config: &BuildConfig,
        file: String,
        file_index: usize,
        api_config: &ApiConfig,
        pdl: &str,
        build_info: BuildInfo,
    ) -> Result<Self, Error> {
        let mut dummy_context = tera::Context::new();
        dummy_context.insert("chunk", "placeholder");

        let mut prompt = messages_from_pdl(
            pdl.to_string(),
            dummy_context,
        )?;

        if let Some(message) = prompt.last_mut() {
            debug_assert_eq!(message.role, Role::User);
            let content = tokens.iter().map(
                |content| MessageContent::from(content.clone())
            ).collect::<Vec<MessageContent>>();

            message.content = content;
        }

        else {
            unreachable!()
        }

        let mut request = ChatRequest {
            api_key: api_config.api_key.clone(),
            messages: prompt,
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
        };
        let mut response = request.send().await?;
        let mut response_text = response.get_message(0).unwrap();
        let json_regex = Regex::new(r"(?s)[^{}]*(\{.*\})[^{}]*").unwrap();

        let mut data = vec![];
        let mut images = vec![];
        let mut char_len = 0;
        let mut image_count = 0;
        let mut mistakes = 0;

        for r in tokens.iter() {
            match r {
                AtomicToken::String { data: s, char_len: n } => {
                    data.push(s.clone());
                    char_len += *n;
                },
                AtomicToken::Image(i) => {
                    images.push(i.key.clone());
                    image_count += 1;
                    data.push(format!("img_{}", i.key));
                },
            }
        }

        let data = data.concat();

        let (title, summary) = loop {
            let error_message;

            if let Some(cap) = json_regex.captures(&response_text) {
                let json_text = cap[1].to_string();

                match json::parse(&json_text) {
                    Ok(j) => match j {
                        JsonValue::Object(obj) if obj.len() == 2 => match (
                            obj.get("title"), obj.get("summary")
                        ) {
                            (Some(title), Some(summary)) => match (title.as_str(), summary.as_str()) {
                                (Some(title), Some(summary)) => {
                                    let summary_len = summary.chars().count();

                                    if summary_len < config.min_summary_len && char_len + image_count * config.image_size > config.min_summary_len * 2 {
                                        error_message = format!("Your summary is too short. Make sure that it's at least {} characters long.", config.min_summary_len);
                                    }

                                    else if summary_len > config.max_summary_len {
                                        error_message = format!("Your summary is too long. Make sure that it's less than {} characters long.", config.max_summary_len);
                                    }

                                    else if title.contains("\n") || summary.contains("\n") {
                                        error_message = format!("Your output has a correct schema, but please don't include newline characters in your output.");
                                    }

                                    else {
                                        break (title.to_string(), summary.to_string());
                                    }
                                },
                                _ => {
                                    error_message = String::from("Give me a json object with 2 keys: \"title\" and \"summary\". Make sure that both are string.");
                                },
                            },
                            _ => {
                                error_message = String::from("Give me a json object with 2 keys: \"title\" and \"summary\".");
                            }, 
                        },
                        JsonValue::Object(_) => {
                            error_message = String::from("Please give me a json object that contains 2 keys: \"title\" and \"summary\". Don't add keys to give extra information, put all your information in those two fields.");
                        },
                        _ => {
                            error_message = String::from("Give me a json object with 2 keys: \"title\" and \"summary\".");
                        },
                    },
                    Err(_) => {
                        error_message = String::from("I cannot parse your output. It seems like your output is not a valid json. Please give me a valid json.");
                    },
                }
            }

            else {
                error_message = String::from("I cannot find curly braces in your response. Please give me a valid json output.");
            }

            mistakes += 1;

            // if a model is too stupid, it cannot create title and summary
            if mistakes > 5 {
                let data_chars = data.chars().collect::<Vec<_>>();

                // default values
                break (
                    String::from("untitled"),
                    data_chars[..((config.min_summary_len + config.max_summary_len) / 2).min(data_chars.len())].into_iter().collect::<String>().replace("\n", " "),
                );
            }

            request.messages.push(Message {
                role: Role::Assistant,
                content: MessageContent::simple_message(response_text.to_string()),
            });
            request.messages.push(Message {
                role: Role::User,
                content: MessageContent::simple_message(error_message),
            });
            response = request.send().await?;
            response_text = response.get_message(0).unwrap();
        };
        let mut hasher = Sha3_256::new();
        hasher.update(data.as_bytes());
        hasher.update(title.as_bytes());
        hasher.update(summary.as_bytes());
        let uid = hasher.finalize();

        Ok(Chunk {
            data,
            images,
            char_len,
            image_count,
            title,
            summary,
            file: normalize(&file)?,
            index: file_index,
            uid: format!("{uid:064x}"),
            build_info,
            external_base: None,
        })
    }

    pub fn len(&self) -> usize {
        // TODO: it has to be
        // `self.data.chars().count() + self.image_count * (config.image_size)`,
        // but `config.image_size` is not available
        self.data.chars().count()
    }
}

// TODO: merging chunks is not tested yet
pub fn merge_and_convert_chunks(chunks: Vec<Chunk>) -> Vec<RenderableChunk> {
    let mut merge_candidates = HashSet::new();
    let mut curr_chunks = HashMap::new();

    for chunk in chunks.into_iter() {
        merge_candidates.insert((chunk.file.clone(), chunk.index + 1));
        let _d = curr_chunks.insert((chunk.file.clone(), chunk.index), chunk).is_none();
        debug_assert!(_d);
    }

    // it has to merge from left to right
    let mut merge_candidates: Vec<_> = merge_candidates.into_iter().collect();
    merge_candidates.sort_by_key(|(_, index)| *index);

    for candidate in merge_candidates.iter() {
        if curr_chunks.contains_key(candidate) {
            let pre = curr_chunks.remove(&(candidate.0.clone(), candidate.1 - 1)).unwrap();
            let post = curr_chunks.remove(candidate).unwrap();
            curr_chunks.insert((candidate.0.clone(), candidate.1), merge_chunks(pre, post));

            return merge_and_convert_chunks(curr_chunks.into_values().collect());
        }
    }

    curr_chunks.into_iter().map(|(_, c)| c.into()).collect()
}

fn merge_chunks(pre: Chunk, post: Chunk) -> Chunk {
    debug_assert_eq!(pre.file, post.file);
    debug_assert_eq!(pre.index + 1, post.index);
    let Chunk {
        data: data_pre,
        images: images_pre,
        ..
    } = pre;
    let Chunk {
        data: data_post,
        images: images_post,
        file,
        index,
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
        file,
        index,

        // TODO: is it okay to leave these fields empty?
        summary: String::new(),
        title: String::new(),
        uid: Uid::new(),
        build_info: BuildInfo::dummy(),
        external_base: None,
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

// `f` is run on a chunk, not on an array of chunks
// make sure to back up files before running this
pub fn update_chunk_schema<F: Fn(JsonValue) -> Result<JsonValue, Error>>(
    index_dirs: Vec<String>,
    f: &F,
) -> Result<(), Error> {
    for dir in index_dirs.iter() {
        let index = Index::load(dir.to_string(), true)?;
        index.map_chunk_jsons(f)?;
    }

    Ok(())
}
