use chrono::offset::Local;
use crate::ApiConfig;
use crate::error::Error;
use crate::index::{BuildConfig, Index, tfidf};
use crate::index::file::AtomicToken;
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
    create_dir_all,
    exists,
    join,
    normalize,
    parent,
    read_bytes,
    set_extension,
    write_bytes,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::collections::{HashMap, HashSet};
use std::io::Read;

mod build_info;
mod renderable;

pub use build_info::ChunkBuildInfo;
pub use renderable::RenderableChunk;

// I wanted it to be u128, but serde_json does not support u128
pub type Uid = String;
pub type Path = String;
pub const CHUNK_DIR_NAME: &str = "chunks";

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Chunk {
    pub data: String,

    /// It's both key and path of an image.
    /// For example, if "abcdef" is in `images`,
    /// it replaces "img_abcdef" in `data` with `.ragit/images/abcdef.png`
    pub images: Vec<String>,
    pub char_len: usize,

    /// It's not always `images.len()`. If the same image appears twice,
    /// it's pushed only once to `images` but twice to `image_count`
    pub image_count: usize,

    pub title: String,
    pub summary: String,

    pub file: String,  // rel path
    pub index: usize,  // index in the file
    pub uid: Uid,
    pub build_info: ChunkBuildInfo,
    pub timestamp: i64,

    /// If it belongs to an external base, the name of the
    /// base is kept here.
    #[serde(skip)]
    pub external_base: Option<String>,
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
        Some(b) => {
            Err(Error::InvalidChunkPrefix(*b))
        },
        None => {
            // simple hack: it throws the exact error that I want
            serde_json::from_slice::<Chunk>(&[])?;
            unreachable!()
        },
    }
}

/// It also creates a tfidf index of the chunk.
pub fn save_to_file(
    path: &Path,
    chunk: &Chunk,

    // if the result json is bigger than threshold (in bytes), the file is compressed
    compression_threshold: u64,
    compression_level: u32,
    root_dir: &Path,
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
        let mut dummy_context = tera::Context::new();
        dummy_context.insert("chunk", "placeholder");
        dummy_context.insert("previous_summary", &previous_summary.is_some());

        let mut prompt = messages_from_pdl(
            pdl.to_string(),
            dummy_context,
        )?;

        if let Some(message) = prompt.last_mut() {
            if message.role != Role::User {
                return Err(Error::BrokenPrompt(String::from("The last turn of a prompt must be of <|user|>.")));
            }

            let mut content = if let Some(previous_summary) = previous_summary {
                vec![MessageContent::String(format!("Previous Summary: {previous_summary}\n\nChunk: "))]
            } else {
                vec![]
            };

            for chunk_content in tokens.iter().map(
                |content| MessageContent::from(content.clone())
            ) {
                content.push(chunk_content);
            }

            message.content = content;
        }

        else {
            return Err(Error::BrokenPrompt(String::from("Got an empty prompt.")));
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

        let mut result = Chunk {
            data,
            images,
            char_len,
            image_count,
            title,
            summary,
            file: normalize(&file)?,
            index: file_index,
            uid: String::new(),
            build_info,
            timestamp: Local::now().timestamp(),
            external_base: None,
        };
        let uid_pile = format!("{}{}{}", result.data, result.title, result.summary);
        let mut hasher = Sha3_256::new();
        hasher.update(uid_pile.as_bytes());
        result.uid = format!("{:064x}", hasher.finalize());

        Ok(result)
    }
}

// TODO: merging chunks is not tested yet
pub(crate) fn merge_and_convert_chunks(index: &Index, chunks: Vec<Chunk>) -> Result<Vec<RenderableChunk>, Error> {
    let mut merge_candidates = HashSet::new();
    let mut curr_chunks = HashMap::new();

    for chunk in chunks.into_iter() {
        merge_candidates.insert((chunk.file.clone(), chunk.index + 1));
        assert!(curr_chunks.insert((chunk.file.clone(), chunk.index), chunk).is_none());
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
    assert_eq!(pre.file, post.file);
    assert_eq!(pre.index + 1, post.index);
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
        timestamp: Local::now().timestamp(),

        // TODO: is it okay to leave these fields empty?
        summary: String::new(),
        title: String::new(),
        uid: Uid::new(),
        build_info: ChunkBuildInfo::dummy(),
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

pub(crate) fn is_valid_uid(uid: &Uid) -> bool {
    uid.len() == 64 && uid.chars().all(
        |c| '0' <= c && c <= '9' || 'a' <= c && c <= 'f'
    )
}
