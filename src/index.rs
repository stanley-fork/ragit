use crate::INDEX_DIR_NAME;
use crate::api_config::{ApiConfig, API_CONFIG_FILE_NAME, ApiConfigRaw};
use crate::chunk::{self, BuildInfo, Chunk, CHUNK_DIR_NAME, CHUNK_INDEX_DIR_NAME, Uid};
use crate::error::{Error, JsonType, get_type};
use crate::external::ExternalIndex;
use crate::prompts::{PROMPTS, PROMPT_DIR};
use crate::query::{Keywords, QueryConfig, QUERY_CONFIG_FILE_NAME, extract_keywords};
use json::JsonValue;
use ragit_api::{
    ChatRequest,
    Message,
    MessageContent,
    RecordAt,
    Role,
    encode_base64,
    messages_from_pdl,
};
use ragit_fs::{
    WriteMode,
    create_dir_all,
    diff,
    exists,
    file_name,
    is_dir,
    join,
    join3,
    join4,
    normalize,
    read_bytes,
    read_string,
    remove_file,
    rename,
    set_extension,
    write_bytes,
    write_string,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter;

mod commands;
mod config;
pub mod file;
pub mod tfidf;

pub use commands::{AddMode, AddResult};
pub use config::{BuildConfig, BUILD_CONFIG_FILE_NAME};
pub use file::{FileReader, get_file_hash};
pub use tfidf::{ProcessedDoc, TfIdfResult, TfIdfState, UpdateTfidf, consume_tfidf_file};

pub const CONFIG_DIR_NAME: &str = "configs";
pub const IMAGE_DIR_NAME: &str = "images";
pub const INDEX_FILE_NAME: &str = "index.json";
pub const LOG_DIR_NAME: &str = "logs";

pub type Path = String;
pub type FileHash = String;

// all the `Path` are normalized relative paths
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Index {
    ragit_version: String,
    pub chunk_count: usize,
    pub staged_files: Vec<Path>,
    pub processed_files: HashMap<Path, FileHash>,
    pub curr_processing_file: Option<Path>,
    chunk_files: HashMap<Path, usize>,  // number of chunks in a file

    // the json file only stores links to the external knowledge-bases,
    // but the code actually loads all the externals
    pub external_index_info: Vec<ExternalIndex>,
    #[serde(skip)]
    pub external_indexes: Vec<Index>,

    // it's not used by code, but used by serde
    // users modify json file, which is deserialized to `ApiConfigRaw`,
    // which is then converted to `ApiConfig` by `.init_api_config()`
    #[serde(skip)]
    api_config_raw: ApiConfigRaw,

    #[serde(skip)]
    pub root_dir: Path,
    #[serde(skip)]
    pub build_config: BuildConfig,
    #[serde(skip)]
    pub query_config: QueryConfig,
    #[serde(skip)]
    pub api_config: ApiConfig,
    #[serde(skip)]
    prompts: HashMap<String, String>,
}

/// 1. If you want to do something with chunks, use `LoadMode::QuickCheck`.
/// 2. If you have nothing to do with chunks, use `LoadMode::OnlyJson`.
/// 3. If something's broken and you don't want it to crash, use `LoadMode::Minimum`. It can still crash, though.
/// 4. If you want to be very sure that nothing's broken and you don't care about init-time, use `LoadMode::Check`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LoadMode {
    /// It only loads `index.json`. It doesn't care whether config files prompt files, or chunk files are broken.
    Minimum,

    /// It loads json files, but doesn't care whether the chunk files are broken.
    OnlyJson,

    /// It checks and auto-recovers if `self.curr_processing_file` is not None. If the value is not None,
    /// a previous build was interrupted and something could be broken.
    QuickCheck,

    /// It always checks and auto-recovers. You should be very careful, `check` and `auto-recover` are very expensive.
    Check,
}

impl Index {
    /// It works like git. `root_dir` is the root of the repo. And it creates dir `.rag_index`, like `.git`.
    /// It reads the files in the repo and creates index.
    pub fn new(
        root_dir: Path,
    ) -> Result<Self, Error> {
        let index_dir = join(&root_dir, INDEX_DIR_NAME)?;

        if exists(&index_dir) {
            return Err(Error::IndexAlreadyExists(index_dir));
        }

        create_dir_all(&index_dir)?;
        create_dir_all(&Index::get_rag_path(
            &root_dir,
            &CONFIG_DIR_NAME.to_string(),
        ))?;
        create_dir_all(&Index::get_rag_path(
            &root_dir,
            &CHUNK_DIR_NAME.to_string(),
        ))?;
        create_dir_all(&Index::get_rag_path(
            &root_dir,
            &IMAGE_DIR_NAME.to_string(),
        ))?;
        create_dir_all(&Index::get_rag_path(
            &root_dir,
            &CHUNK_INDEX_DIR_NAME.to_string(),
        ))?;

        let build_config = BuildConfig::default();
        let query_config = QueryConfig::default();
        let api_config = ApiConfig::default();
        let api_config_raw = ApiConfigRaw::default();

        let mut result = Index {
            ragit_version: crate::VERSION.to_string(),
            chunk_count: 0,
            staged_files: vec![],
            processed_files: HashMap::new(),
            curr_processing_file: None,
            build_config,
            query_config,
            api_config_raw,
            api_config,
            root_dir,
            chunk_files: HashMap::new(),
            prompts: PROMPTS.clone(),
            external_index_info: vec![],
            external_indexes: vec![],
        };
        result.create_new_chunk_file()?;

        write_bytes(
            &result.get_build_config_path()?,
            &serde_json::to_vec_pretty(&result.build_config)?,
            WriteMode::AlwaysCreate,
        )?;
        write_bytes(
            &result.get_query_config_path()?,
            &serde_json::to_vec_pretty(&result.query_config)?,
            WriteMode::AlwaysCreate,
        )?;
        write_bytes(
            &result.get_api_config_path()?,
            &serde_json::to_vec_pretty(&result.api_config_raw)?,
            WriteMode::AlwaysCreate,
        )?;
        result.save_to_file()?;

        Ok(result)
    }

    pub fn load(
        root_dir: Path,
        load_mode: LoadMode,
    ) -> Result<Self, Error> {
        let mut result = Index::load_minimum(root_dir)?;

        if load_mode == LoadMode::Minimum {
            return Ok(result);
        }

        result.build_config = serde_json::from_str::<BuildConfig>(
            &read_string(&result.get_build_config_path()?)?,
        )?;
        result.query_config = serde_json::from_str::<QueryConfig>(
            &read_string(&result.get_query_config_path()?)?,
        )?;
        result.api_config_raw = serde_json::from_str::<ApiConfigRaw>(
            &read_string(&result.get_api_config_path()?)?,
        )?;
        result.api_config = result.init_api_config(&result.api_config_raw)?;
        result.load_prompts()?;
        result.load_external_indexes(load_mode)?;

        match load_mode {
            LoadMode::QuickCheck if result.curr_processing_file.is_some() && result.check(false).is_err() => {
                result.auto_recover()?;
                result.save_to_file()?;
                Ok(result)
            },
            LoadMode::Check if result.check(false).is_err() => {
                result.auto_recover()?;
                result.save_to_file()?;
                Ok(result)
            },
            _ => Ok(result),
        }
    }

    /// It only loads `index.json`. No config files, no prompt files, and it doesn't care whether chunk files are broken or not.
    /// It's for `rag check --auto-recover`: it only loads minimum data and the auto-recover function will load or fix the others.
    fn load_minimum(root_dir: Path) -> Result<Self, Error> {
        let index_json = read_string(&Index::get_rag_path(
            &root_dir,
            &INDEX_FILE_NAME.to_string(),
        ))?;

        let mut result = serde_json::from_str::<Index>(&index_json)?;
        result.root_dir = root_dir;

        if result.ragit_version != crate::VERSION {
            // TODO
            // 1. show warning message
            // 2. impl auto-version-migration
        }

        Ok(result)
    }

    pub fn load_or_init(
        root_dir: Path,
    ) -> Result<Self, Error> {
        let index_dir = join(
            &root_dir,
            &INDEX_DIR_NAME.to_string(),
        )?;

        if exists(&index_dir) {
            // `load_or_init` cannot be done in only-json mode, because only-json `init` doesn't make sense
            Index::load(root_dir, LoadMode::QuickCheck)
        }

        else {
            Index::new(root_dir)
        }
    }

    pub fn save_to_file(&self) -> Result<(), Error> {
        self.save_prompts()?;

        Ok(write_bytes(
            &Index::get_rag_path(
                &self.root_dir,
                &INDEX_FILE_NAME.to_string(),
            ),
            &serde_json::to_vec_pretty(self)?,
            WriteMode::CreateOrTruncate,
        )?)
    }

    pub async fn load_chunks_or_tfidf(
        &self,
        query: &str,
        ignored_chunks: Vec<Uid>,
    ) -> Result<Vec<Chunk>, Error> {
        if self.chunk_count + self.count_external_chunks() > self.query_config.max_titles {
            let keywords = extract_keywords(query, &self.api_config, &self.get_prompt("extract_keyword")?).await?;
            let uids = self.run_tfidf(keywords, ignored_chunks)?.into_iter().map(|r| r.id).collect::<Vec<Uid>>();

            self.get_chunks_by_uid(&uids)
        }

        else {
            let mut chunks = vec![];

            for chunk_file in self.chunk_files_real_path() {
                chunks.extend(chunk::load_from_file(&chunk_file)?);
            }

            chunks = chunks.into_iter().filter(
                |chunk| !ignored_chunks.contains(&chunk.uid)
            ).collect();

            Ok(chunks)
        }
    }

    fn chunk_files(&self) -> Vec<Path> {
        self.chunk_files.keys().map(|chunk_file| set_extension(chunk_file, "chunks").unwrap()).collect()
    }

    // Rust doesn't allow me to return `.keys().map()` as `impl Iter<Item = Path>`
    fn chunk_files_real_path(&self) -> Vec<Path> {
        self.chunk_files.keys().map(|chunk_file| Index::get_chunk_path(&self.root_dir, &set_extension(chunk_file, "chunks").unwrap())).collect()
    }

    fn tfidf_files(&self) -> Vec<Path> {
        self.chunk_files().iter().map(|chunk| set_extension(chunk, "tfidf").unwrap()).collect()
    }

    fn tfidf_files_real_path(&self) -> Vec<Path> {
        self.tfidf_files().iter().map(|rel_path| Index::get_chunk_path(&self.root_dir, rel_path)).collect()
    }

    fn load_curr_processing_chunks(&self) -> Result<Vec<Chunk>, Error> {
        chunk::load_from_file(&Index::get_chunk_path(&self.root_dir, &self.get_curr_processing_chunks_path()))
    }

    fn get_curr_processing_chunks_path(&self) -> Path {
        for (path, count) in self.chunk_files.iter() {
            if *count < self.build_config.chunks_per_json {
                return set_extension(path, "chunks").unwrap();
            }
        }

        unreachable!()
    }

    fn create_new_chunk_file(&mut self) -> Result<(), Error> {
        let real_path = Index::get_chunk_path(
            &self.root_dir,

            // hack: a name of a chunk file is xor of its chunks' uids, so an empty chunk file must be 0
            &format!("{:064x}.chunks", 0),
        );
        chunk::save_to_file(
            &real_path,
            &[],
            self.build_config.compression_threshold,
            self.build_config.compression_level,
            &self.root_dir,
            UpdateTfidf::Nop,
        )?;
        self.chunk_files.insert(format!("{:064x}", 0), 0);

        Ok(())
    }

    async fn add_image_description(&self, key: &str) -> Result<(), Error> {
        let description_path = Index::get_image_path(&self.root_dir, key, "json");
        let image_path = Index::get_image_path(&self.root_dir, key, "png");
        let image_bytes = read_bytes(&image_path)?;
        let image_bytes = encode_base64(&image_bytes);

        if let Ok(j) = read_string(&description_path) {
            if json::parse(&j).is_ok() {
                return Ok(());
            }

            else {
                remove_file(&description_path)?;
            }
        }

        let mut context = tera::Context::new();
        context.insert("image_type", "png");
        context.insert("image_bytes", &image_bytes);
        let pdl = self.get_prompt("describe_image")?;
        let messages = messages_from_pdl(
            pdl.to_string(),
            context,
        )?;
        let mut mistakes = 0;

        let mut request = ChatRequest {
            messages,
            api_key: self.api_config.api_key.clone(),
            model: self.api_config.model,
            frequency_penalty: None,
            max_tokens: None,
            max_retry: self.api_config.max_retry,
            sleep_between_retries: self.api_config.sleep_between_retries,
            timeout: self.api_config.timeout,
            temperature: None,
            record_api_usage_at: self.api_config.dump_api_usage_at.clone().map(
                |path| RecordAt { path, id: String::from("describe_image") }
            ),
            dump_pdl_at: self.api_config.create_pdl_path("describe_image"),
        };
        let mut response = request.send().await?;
        let mut response_text = response.get_message(0).unwrap();
        let json_regex = Regex::new(r"(?s)[^{}]*(\{.*\})[^{}]*").unwrap();

        let result = loop {
            let error_message;

            if let Some(cap) = json_regex.captures(&response_text) {
                let json_text = cap[1].to_string();

                match json::parse(&json_text) {
                    Ok(j) => match j {
                        JsonValue::Object(ref obj) if obj.len() == 2 => match (
                            obj.get("extracted_text"), obj.get("explanation"),
                        ) {
                            (Some(extracted), Some(explanation)) => match (extracted.as_str(), explanation.as_str()) {
                                (Some(_), Some(_)) => {
                                    break j.clone();
                                },
                                _ => {
                                    error_message = String::from("Please make sure that both values of the object are string.");
                                },
                            },
                            _ => {
                                error_message = String::from("Give me a json object with 2 keys: \"extracted_text\" and \"explanation\". Make sure that both are string.");
                            },
                        },
                        _ => {
                            error_message = String::from("Give me a json object with 2 keys: \"extracted_text\" and \"explanation\". Make sure that both are string.");
                        },
                    },
                    _ => {
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
                break vec![("extracted_text", ""), ("explanation", "")].into_iter().collect::<HashMap<&str, &str>>().into();
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

        write_string(
            &description_path,
            &result.pretty(4),
            WriteMode::AlwaysCreate,
        )?;

        Ok(())
    }

    pub fn run_tfidf(
        &self,
        keywords: Keywords,

        // 1. why not HashSet<Uid>?
        // 2. for now, no code does something with this value
        ignored_chunks: Vec<Uid>,
    ) -> Result<Vec<TfIdfResult<Uid>>, Error> {
        let mut tfidf_state = TfIdfState::new(&keywords);
        self.generate_tfidfs()?;

        for tfidf_file in self.tfidf_files_real_path() {
            consume_tfidf_file(
                tfidf_file,
                &ignored_chunks,
                &mut tfidf_state,
            )?;
        }

        for external_index in self.external_indexes.iter() {
            external_index.generate_tfidfs()?;

            for tfidf_file in external_index.tfidf_files_real_path() {
                consume_tfidf_file(
                    tfidf_file,
                    &ignored_chunks,
                    &mut tfidf_state,
                )?;
            }
        }

        Ok(tfidf_state.get_top(self.query_config.max_summaries))
    }

    // input and output has the same order
    // if any uid is missing, it returns Err
    pub fn get_chunks_by_uid(&self, uids: &[Uid]) -> Result<Vec<Chunk>, Error> {
        let mut visited_files = HashSet::new();
        let mut chunk_map = HashMap::with_capacity(uids.len());

        for uid in uids.iter() {
            let (root_dir, chunk_file) = self.get_chunk_file_by_index(uid)?;
            let chunk_file_real_path = Index::get_chunk_path(&root_dir, &set_extension(&chunk_file, "chunks")?);

            if visited_files.contains(&chunk_file) {
                continue;
            }

            let chunks = chunk::load_from_file(&chunk_file_real_path)?;
            visited_files.insert(chunk_file);

            for mut chunk in chunks.into_iter() {
                if uids.contains(&chunk.uid) {
                    chunk.external_base = Some(root_dir.clone());
                    chunk_map.insert(chunk.uid.clone(), chunk);
                }
            }
        }

        let mut result = Vec::with_capacity(uids.len());

        for uid in uids.iter() {
            match chunk_map.get(uid) {
                Some(chunk) => {
                    result.push(chunk.clone());
                },
                None => {
                    return Err(Error::NoSuchChunk {
                        uid: uid.clone(),
                    });
                },
            }
        }

        Ok(result)
    }

    pub fn get_tfidf_by_chunk_uid(
        &self,
        uid: Uid,
    ) -> Result<ProcessedDoc, Error> {
        self.generate_tfidfs()?;
        let (root_dir, chunk_file) = self.get_chunk_file_by_index(&uid)?;
        let chunk_file_real_path = Index::get_chunk_path(&root_dir, &set_extension(&chunk_file, "chunks")?);
        let tfidf_file_real_path = set_extension(&chunk_file_real_path, "tfidf")?;

        let tfidfs = tfidf::load_from_file(&tfidf_file_real_path)?;

        for processed_doc in tfidfs.iter() {
            if processed_doc.chunk_uid.as_ref() == Some(&uid) {
                return Ok(processed_doc.clone());
            }
        }

        Err(Error::NoSuchChunk { uid })
    }

    // it loads all the chunks that belongs to this index, runs `f` on them, and save them to the file.
    // it's useful when the schema of chunks have changed
    // make sure to backup files before running this!
    // it runs on chunks, not on an array of chunks
    pub fn map_chunk_jsons<F: Fn(JsonValue) -> Result<JsonValue, Error>>(&self, f: &F) -> Result<(), Error> {
        for chunk_file in self.chunk_files_real_path() {
            let raw = read_string(&chunk_file)?;
            let j = json::parse(&raw)?;

            match j {
                JsonValue::Array(chunks) => {
                    let mut new_chunks = vec![];

                    for chunk in chunks.into_iter() {
                        new_chunks.push(f(chunk)?);
                    }

                    write_string(
                        &chunk_file,
                        &JsonValue::from(new_chunks).pretty(4),
                        WriteMode::CreateOrTruncate,
                    )?;
                },
                _ => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Array,
                        got: get_type(&j),
                    });
                },
            }
        }

        Ok(())
    }

    // every path in index.json are relative path to root_dir
    fn get_rag_path(root_dir: &Path, rel_path: &Path) -> Path {
        normalize(
            &join3(
                root_dir,
                &INDEX_DIR_NAME.to_string(),
                rel_path,
            ).unwrap(),
        ).unwrap()
    }

    pub(crate) fn get_data_path(root_dir: &Path, rel_path: &Path) -> Path {
        normalize(
            &join(
                root_dir,
                rel_path,
            ).unwrap(),
        ).unwrap()
    }

    pub(crate) fn get_rel_path(root_dir: &Path, real_path: &Path) -> Path {
        normalize(
            &diff(
                real_path,
                root_dir,
            ).unwrap(),
        ).unwrap()
    }

    fn get_chunk_path(root_dir: &Path, chunk_name: &Path) -> Path {
        normalize(
            &join4(
                root_dir,
                &INDEX_DIR_NAME.to_string(),
                &CHUNK_DIR_NAME.to_string(),
                chunk_name,
            ).unwrap(),
        ).unwrap()
    }

    fn get_image_path(root_dir: &Path, image_key: &str, extension: &str) -> Path {
        normalize(
            &join4(
                root_dir,
                &INDEX_DIR_NAME.to_string(),
                &IMAGE_DIR_NAME.to_string(),
                &set_extension(image_key, extension).unwrap(),
            ).unwrap(),
        ).unwrap()
    }

    fn get_chunk_index_path(root_dir: &Path, chunk_uid: &Uid) -> Path {
        normalize(
            &join4(
                root_dir,
                &INDEX_DIR_NAME.to_string(),
                &CHUNK_INDEX_DIR_NAME.to_string(),
                &format!("{}.json", chunk_uid.get(0..2).unwrap()),
            ).unwrap(),
        ).unwrap()
    }

    fn get_api_config_path(&self) -> Result<Path, Error> {
        Ok(Index::get_rag_path(
            &self.root_dir,
            &join(
                CONFIG_DIR_NAME,
                API_CONFIG_FILE_NAME,
            )?,
        ))
    }

    fn get_build_config_path(&self) -> Result<Path, Error> {
        Ok(Index::get_rag_path(
            &self.root_dir,
            &join(
                CONFIG_DIR_NAME,
                BUILD_CONFIG_FILE_NAME,
            )?,
        ))
    }

    fn get_query_config_path(&self) -> Result<Path, Error> {
        Ok(Index::get_rag_path(
            &self.root_dir,
            &join(
                CONFIG_DIR_NAME,
                QUERY_CONFIG_FILE_NAME,
            )?,
        ))
    }

    pub(crate) fn init_api_config(&self, raw: &ApiConfigRaw) -> Result<ApiConfig, Error> {
        let dump_log_at = if raw.dump_log {
            let path = Index::get_rag_path(
                &self.root_dir,
                &LOG_DIR_NAME.to_string(),
            );

            if !exists(&path) || !is_dir(&path) {
                create_dir_all(&path)?;
            }

            Some(path)
        }

        else {
            None
        };

        let dump_api_usage_at = if raw.dump_api_usage {
            let path = Index::get_rag_path(
                &self.root_dir,
                &"usages.json".to_string(),
            );

            if !exists(&path) || is_dir(&path) {
                write_string(
                    &path,
                    "{}",
                    WriteMode::AlwaysCreate,
                )?;
            }

            Some(path)
        }

        else {
            None
        };

        Ok(ApiConfig {
            api_key: raw.api_key.clone(),
            max_retry: raw.max_retry,
            model: raw.model.parse()?,
            timeout: raw.timeout,
            sleep_after_llm_call: raw.sleep_after_llm_call,
            sleep_between_retries: raw.sleep_between_retries,
            dump_log_at,
            dump_api_usage_at,
        })
    }

    // TODO: there must be an API function that modifies prompts
    pub(crate) fn load_prompts(&mut self) -> Result<(), Error> {
        for prompt_name in PROMPTS.keys() {
            let prompt_path = Index::get_rag_path(
                &self.root_dir,
                &join(
                    PROMPT_DIR,
                    &set_extension(
                        prompt_name,
                        "pdl",
                    )?,
                )?,
            );

            match read_string(&prompt_path) {
                Ok(p) => {
                    self.prompts.insert(prompt_name.to_string(), p);
                },
                Err(_) => {
                    println!("Warning: failed to load `{prompt_name}.pdl`");
                },
            }
        }

        Ok(())
    }

    pub(crate) fn save_prompts(&self) -> Result<(), Error> {
        let prompt_real_dir = Index::get_rag_path(
            &self.root_dir,
            &PROMPT_DIR.to_string(),
        );

        if !exists(&prompt_real_dir) {
            create_dir_all(&prompt_real_dir)?;
        }

        for (prompt_name, prompt) in self.prompts.iter() {
            let prompt_path = join(
                &prompt_real_dir,
                &set_extension(
                    prompt_name,
                    "pdl",
                )?,
            )?;

            write_string(
                &prompt_path,
                prompt,
                WriteMode::CreateOrTruncate,
            )?;
        }

        Ok(())
    }

    pub fn get_prompt(&self, prompt_name: &str) -> Result<String, Error> {
        match self.prompts.get(prompt_name) {
            Some(prompt) => Ok(prompt.to_string()),
            None => Err(Error::PromptMissing(prompt_name.to_string())),
        }
    }

    pub fn remove_chunks_by_file_name(&mut self, file: Path /* rel_path */ ) -> Result<(), Error> {
        let mut total_chunk_count = 0;

        for chunk_path in self.chunk_files() {
            let real_path = Index::get_chunk_path(
                &self.root_dir,
                &chunk_path,
            );
            let mut chunks = chunk::load_from_file(&real_path)?;
            let mut chunks_to_remove = vec![];
            let old_chunk_file_name = file_name(&chunk_path)?;
            let mut new_chunk_file_name = file_name(&chunk_path)?;

            for chunk in chunks.iter() {
                if chunk.file == file {
                    chunks_to_remove.push(chunk.uid.clone());
                    self.remove_chunk_index(&chunk.uid)?;
                    new_chunk_file_name = xor_sha3(
                        &chunk.uid,
                        &new_chunk_file_name,
                    )?;
                }
            }

            if chunks_to_remove.is_empty() {
                total_chunk_count += chunks.len();
                continue;
            }

            chunks = chunks.into_iter().filter(
                |chunk| !chunks_to_remove.contains(&chunk.uid)
            ).collect();
            chunk::save_to_file(
                &real_path,
                &chunks,
                self.build_config.compression_threshold,
                self.build_config.compression_level,
                &self.root_dir,
                UpdateTfidf::Remove,
            )?;
            self.chunk_files.insert(file_name(&chunk_path)?, chunks.len());
            total_chunk_count += chunks.len();

            // a name of a chunk file is an xor of its chunks
            // so a chunk file has to be renamed when a chunk is added/removed
            self.rename_chunk_file(
                &old_chunk_file_name,
                &new_chunk_file_name,
            )?;
        }

        self.chunk_count = total_chunk_count;
        Ok(())
    }

    // It returns `(external_index's root_dir, rel_chunk_path)` because the chunk might be at an external knowledge-base
    pub(crate) fn get_chunk_file_by_index(&self, chunk_uid: &Uid) -> Result<(Path, Path), Error> {
        for knowledge_base in iter::once(&self.root_dir).chain(self.external_indexes.iter().map(|index| &index.root_dir)) {
            let chunk_index_file = Index::get_chunk_index_path(knowledge_base, chunk_uid);

            if !exists(&chunk_index_file) {
                continue;
            }

            let json_content = read_string(&chunk_index_file)?;
            let j = json::parse(&json_content)?;

            match j {
                JsonValue::Object(obj) => match obj.get(chunk_uid) {
                    Some(path) => match path.as_str() {
                        Some(path) => {
                            return Ok((knowledge_base.to_string(), path.to_string()));
                        },
                        None => {
                            return Err(Error::JsonTypeError {
                                expected: JsonType::String,
                                got: get_type(path),
                            });
                        },
                    },
                    None => {},
                },
                _ => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: get_type(&j),
                    });
                },
            }
        }

        Err(Error::NoSuchChunk { uid: chunk_uid.clone() })
    }

    pub(crate) fn add_chunk_index(
        &mut self,
        chunk_uid: &Uid,
        chunk_file: &Path,  // can be real_path or rel_path. doesn't matter
        update_chunk_file_name: bool,
    ) -> Result<(), Error> {
        let chunk_index_file = Index::get_chunk_index_path(&self.root_dir, chunk_uid);
        let chunk_file_name = file_name(chunk_file)?;

        let mut index = if !exists(&chunk_index_file) {
            json::object::Object::new()
        }

        else {
            let json_content = read_string(&chunk_index_file)?;
            let j = json::parse(&json_content)?;

            match j {
                JsonValue::Object(obj) => obj,
                _ => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: get_type(&j),
                    });
                },
            }
        };

        index.insert(chunk_uid, chunk_file_name.clone().into());
        write_string(
            &chunk_index_file,
            &JsonValue::Object(index).pretty(4),
            WriteMode::CreateOrTruncate,
        )?;

        // a name of a chunk file is an xor of its chunks
        // so a chunk file has to be renamed when a chunk is added/removed
        if update_chunk_file_name {
            self.rename_chunk_file(
                &chunk_file_name,
                &xor_sha3(
                    &chunk_file_name,
                    &chunk_uid,
                )?,
            )?;
        }

        Ok(())
    }

    pub(crate) fn remove_chunk_index(&self, chunk_uid: &Uid) -> Result<(), Error> {
        let chunk_index_file = Index::get_chunk_index_path(&self.root_dir, chunk_uid);
        let json_content = read_string(&chunk_index_file)?;
        let mut j = json::parse(&json_content)?;

        match &mut j {
            JsonValue::Object(ref mut obj) => match obj.remove(chunk_uid) {
                Some(_) => {},
                None => {
                    return Err(Error::NoSuchChunk { uid: chunk_uid.clone() });
                },
            },
            _ => {
                return Err(Error::JsonTypeError {
                    expected: JsonType::Object,
                    got: get_type(&j),
                });
            },
        }

        write_string(
            &chunk_index_file,
            &j.pretty(4),
            WriteMode::CreateOrTruncate,
        )?;
        Ok(())
    }

    fn rename_chunk_file(
        &mut self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), Error> {
        let old_real_path = Index::get_chunk_path(
            &self.root_dir,
            &set_extension(&old_name, "chunks")?,
        );
        let chunks = chunk::load_from_file(&old_real_path)?;

        for chunk in chunks.iter() {
            let chunk_index_file = Index::get_chunk_index_path(&self.root_dir, &chunk.uid);
            let json_content = read_string(&chunk_index_file)?;
            let j = json::parse(&json_content)?;
            let mut index = match j {
                JsonValue::Object(obj) => obj,
                _ => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: get_type(&j),
                    });
                },
            };
            index.insert(&chunk.uid, new_name.into());
            write_string(
                &chunk_index_file,
                &JsonValue::Object(index).pretty(4),
                WriteMode::CreateOrTruncate,
            )?;
        }

        self.chunk_files.remove(&old_name.to_string());
        self.chunk_files.insert(new_name.to_string(), chunks.len());
        let new_real_path = Index::get_chunk_path(
            &self.root_dir,
            &set_extension(&new_name, "chunks")?,
        );
        rename(&old_real_path, &new_real_path)?;
        let old_tfidf_path = set_extension(&old_real_path, "tfidf")?;

        if exists(&old_tfidf_path) {
            rename(
                &old_tfidf_path,
                &set_extension(&new_real_path, "tfidf")?,
            )?;
        }

        Ok(())
    }

    fn count_external_chunks(&self) -> usize {
        self.external_indexes.iter().map(|index| index.chunk_count).sum()
    }

    pub fn load_image_by_key(&self, key: &str) -> Result<Vec<u8>, Error> {
        Ok(read_bytes(&Index::get_image_path(&self.root_dir, key, "png"))?)
    }

    // tfidf files are kinda lazily-loaded.
    // 1. In order to run tfidf queries, all the tfidf files must be complete.
    //    - Each chunk must have its ProcessedDoc.
    // 2. A tfidf file either 1) not exist at all or 2) is complete.
    // 3. `self.build()` generates most tfidf files, but some would be missing due to performance reasons.
    // 4. It generates missing tfidf files, if exist.
    fn generate_tfidfs(&self) -> Result<(), Error> {
        for chunk_file in self.chunk_files_real_path() {
            let tfidf_file = set_extension(&chunk_file, "tfidf")?;

            if !exists(&tfidf_file) {
                let chunks = chunk::load_from_file(&chunk_file)?;
                chunk::save_to_file(
                    &chunk_file,
                    &chunks,
                    self.build_config.compression_threshold,
                    self.build_config.compression_level,
                    &self.root_dir,
                    UpdateTfidf::Generate,
                )?;
            }
        }

        Ok(())
    }
}

/// It loads `.rag_llama_index.json`, modifies it and saves it.
/// It's useful when the schema of the index has changed.
/// Make sure to backup files before running this!
pub fn update_index_schema<F: Fn(JsonValue) -> Result<JsonValue, Error>>(path: &str, f: &F) -> Result<(), Error> {
    let raw = read_string(path)?;
    let j = json::parse(&raw)?;
    let j_new = f(j)?;
    write_string(
        path,
        &j_new.pretty(4),
        WriteMode::CreateOrTruncate,
    )?;

    Ok(())
}

fn xor_sha3(
    hash1: &str,
    hash2: &str,
) -> Result<String, Error> {
    if hash1.len() != 64 {
        return Err(Error::BrokenHash(hash1.to_string()));
    }

    if hash2.len() != 64 {
        return Err(Error::BrokenHash(hash2.to_string()));
    }

    let mut strings = Vec::with_capacity(8);

    for i in 0..8 {
        match (
            hash1.get((i * 8)..(i * 8 + 8)),
            hash2.get((i * 8)..(i * 8 + 8)),
        ) {
            (Some(h1), Some(h2)) => match (
                u32::from_str_radix(h1, 16),
                u32::from_str_radix(h2, 16),
            ) {
                (Ok(n1), Ok(n2)) => {
                    strings.push(format!("{:08x}", n1 ^ n2));
                },
                (Err(_), _) => {
                    return Err(Error::BrokenHash(hash1.to_string()));
                },
                (_, Err(_)) => {
                    return Err(Error::BrokenHash(hash2.to_string()));
                },
            },
            (None, _) => {
                return Err(Error::BrokenHash(hash1.to_string()));
            },
            (_, None) => {
                return Err(Error::BrokenHash(hash2.to_string()));
            },
        }
    }

    Ok(strings.concat())
}
