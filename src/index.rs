use crate::INDEX_DIR_NAME;
use crate::api_config::{ApiConfig, API_CONFIG_FILE_NAME, ApiConfigRaw};
use crate::chunk::{self, Chunk, ChunkBuildInfo, CHUNK_DIR_NAME};
use crate::error::Error;
use crate::external::ExternalIndex;
use crate::prompts::{PROMPTS, PROMPT_DIR};
use crate::query::{Keywords, QueryConfig, QUERY_CONFIG_FILE_NAME, extract_keywords};
use crate::uid::{self, Uid};
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
    current_dir,
    diff,
    exists,
    extension,
    is_dir,
    join,
    join3,
    join4,
    normalize,
    parent,
    read_bytes,
    read_dir,
    read_string,
    remove_file,
    set_extension,
    write_bytes,
    write_string,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

mod commands;
mod config;
pub mod file;
pub mod tfidf;

pub use commands::{AddMode, AddResult, METADATA_FILE_NAME, RecoverResult, RenderableFile, RenderableModel};
pub use config::{BuildConfig, BUILD_CONFIG_FILE_NAME};
pub use file::{FileReader, get_file_uid};
pub use tfidf::{ProcessedDoc, TfIdfResult, TfIdfState, consume_tfidf_file};

pub const CONFIG_DIR_NAME: &str = "configs";
pub const IMAGE_DIR_NAME: &str = "images";
pub const FILE_INDEX_DIR_NAME: &str = "files";
pub const INDEX_FILE_NAME: &str = "index.json";
pub const LOG_DIR_NAME: &str = "logs";

pub type Path = String;

// all the `Path` are normalized relative paths
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Index {
    ragit_version: String,
    pub chunk_count: usize,
    pub staged_files: Vec<Path>,
    pub processed_files: HashMap<Path, Uid>,
    pub curr_processing_file: Option<Path>,
    repo_url: Option<String>,

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
    /// It works like git. `root_dir` is the root of the repo. And it creates dir `.ragit/`, like `.git/`.
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
            &FILE_INDEX_DIR_NAME.to_string(),
        ))?;

        let build_config = BuildConfig::default();
        let query_config = QueryConfig::default();
        let api_config = ApiConfig::default();
        let api_config_raw = ApiConfigRaw::default();

        let result = Index {
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
            repo_url: None,
            prompts: PROMPTS.clone(),
            external_index_info: vec![],
            external_indexes: vec![],
        };

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
            LoadMode::QuickCheck if result.curr_processing_file.is_some() => {
                result.recover()?;
                result.save_to_file()?;
                Ok(result)
            },
            LoadMode::Check if result.curr_processing_file.is_some() || result.check(false).is_err() => {
                result.recover()?;
                result.save_to_file()?;
                Ok(result)
            },
            _ => Ok(result),
        }
    }

    /// It only loads `index.json`. No config files, no prompt files, and it doesn't care whether chunk files are broken or not.
    /// It's for `rag check --recover`: it only loads minimum data and the recover function will load or fix the others.
    fn load_minimum(root_dir: Path) -> Result<Self, Error> {
        let index_json = read_string(&Index::get_rag_path(
            &root_dir,
            &INDEX_FILE_NAME.to_string(),
        ))?;

        let mut result = serde_json::from_str::<Index>(&index_json)?;
        result.root_dir = root_dir;

        if result.ragit_version != crate::VERSION {
            // TODO: what here?
            // 1. a user prompt asking to run migration
            // 2. ignore. the user has to explicitly run migration
            // 3. a warning message, but no action. the user has to explicitly run migration
            // 4. always run migration
            //
            // The problem is that
            // 1. a version mismatch would be very often.
            // 2. a compatibility issue would be very rare.
            // 3. it's not always possible for the client to tell whether there's an issue or not
            // 4. migration is expensive
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
            let tfidf_results = self.run_tfidf(
                keywords,
                ignored_chunks,
                self.query_config.max_summaries,
            )?;
            let mut chunks = Vec::with_capacity(tfidf_results.len());

            for tfidf_result in tfidf_results.into_iter() {
                let (external_index, uid) = tfidf_result.id;

                match external_index {
                    Some(i) => {
                        chunks.push(self.get_external_base(&i)?.get_chunk_by_uid(uid)?);
                    },
                    None => { chunks.push(self.get_chunk_by_uid(uid)?); },
                }
            }

            Ok(chunks)
        }

        else {
            let mut chunks = vec![];

            for chunk_path in self.get_all_chunk_files()? {
                chunks.push(chunk::load_from_file(&chunk_path)?);
            }

            chunks = chunks.into_iter().filter(
                |chunk| !ignored_chunks.contains(&chunk.uid)
            ).collect();

            Ok(chunks)
        }
    }

    pub fn get_all_chunk_files(&self) -> Result<Vec<Path>, Error> {
        let mut result = vec![];

        for internal in read_dir(&join3(&self.root_dir, &INDEX_DIR_NAME, &CHUNK_DIR_NAME)?)? {
            if !is_dir(&internal) {
                continue;
            }

            for chunk_file in read_dir(&internal)? {
                if extension(&chunk_file).unwrap_or(None).unwrap_or(String::new()) == "chunk" {
                    result.push(chunk_file.to_string());
                }
            }
        }

        Ok(result)
    }

    pub fn get_all_tfidf_files(&self) -> Result<Vec<Path>, Error> {
        let mut result = vec![];

        for internal in read_dir(&join3(&self.root_dir, &INDEX_DIR_NAME, &CHUNK_DIR_NAME)?)? {
            if !is_dir(&internal) {
                continue;
            }

            for tfidf_file in read_dir(&internal)? {
                if extension(&tfidf_file).unwrap_or(None).unwrap_or(String::new()) == "tfidf" {
                    result.push(tfidf_file.to_string());
                }
            }
        }

        Ok(result)
    }

    fn file_index_real_path(&self) -> Result<Vec<Path>, Error> {
        let mut result = vec![];

        for internal in read_dir(&join3(&self.root_dir, &INDEX_DIR_NAME, &FILE_INDEX_DIR_NAME)?)? {
            if !is_dir(&internal) {
                continue;
            }

            for file_index in read_dir(&internal)? {
                result.push(file_index.to_string());
            }
        }

        Ok(result)
    }

    async fn add_image_description(&self, key: &str) -> Result<(), Error> {
        let description_path = Index::get_image_path(&self.root_dir, key, "json");
        let image_path = Index::get_image_path(&self.root_dir, key, "png");
        let image_bytes = read_bytes(&image_path)?;
        let image_bytes = encode_base64(&image_bytes);

        if let Ok(j) = read_string(&description_path) {
            if serde_json::from_str::<Value>(&j).is_ok() {
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

                match serde_json::from_str::<Value>(&json_text) {
                    Ok(j) => match j {
                        Value::Object(ref obj) if obj.len() == 2 => match (
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
                break vec![
                    (String::from("extracted_text"), Value::String(String::new())),
                    (String::from("explanation"), Value::String(String::new())),
                ].into_iter().collect::<Value>();
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

        write_bytes(
            &description_path,
            &serde_json::to_vec_pretty(&result)?,
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
        chunk_count: usize,
    ) -> Result<Vec<TfIdfResult<(Option<ExternalIndex>, Uid)>>, Error> {
        let mut tfidf_state = TfIdfState::new(&keywords);

        for tfidf_file in self.get_all_tfidf_files()? {
            consume_tfidf_file(
                None,  // not an external knowledge_base
                tfidf_file,
                &ignored_chunks,
                &mut tfidf_state,
            )?;
        }

        for (i, external_index) in self.external_indexes.iter().enumerate() {
            for tfidf_file in external_index.get_all_tfidf_files()? {
                consume_tfidf_file(
                    Some(self.external_index_info[i].clone()),
                    tfidf_file,
                    &ignored_chunks,
                    &mut tfidf_state,
                )?;
            }
        }

        Ok(tfidf_state.get_top(chunk_count))
    }

    pub fn get_chunk_by_uid(&self, uid: Uid) -> Result<Chunk, Error> {
        for root_dir in std::iter::once(&self.root_dir).chain(
            self.external_indexes.iter().map(|index| &index.root_dir)
        ) {
            let chunk_at = Index::get_chunk_path(root_dir, uid);

            if exists(&chunk_at) {
                return Ok(chunk::load_from_file(&chunk_at)?);
            }
        }

        Err(Error::NoSuchChunk(uid))
    }

    pub fn get_tfidf_by_chunk_uid(
        &self,
        uid: Uid,
    ) -> Result<ProcessedDoc, Error> {
        for root_dir in std::iter::once(&self.root_dir).chain(
            self.external_indexes.iter().map(|index| &index.root_dir)
        ) {
            let tfidf_at = set_extension(&Index::get_chunk_path(root_dir, uid), "tfidf")?;

            if exists(&tfidf_at) {
                return Ok(tfidf::load_from_file(&tfidf_at)?);
            }
        }

        Err(Error::NoSuchChunk(uid))
    }

    pub fn get_tfidf_by_file_uid(
        &self,
        uid: Uid,
    ) -> Result<ProcessedDoc, Error> {
        let chunk_uids = self.get_chunks_of_file(uid)?;
        let mut result = ProcessedDoc::empty();

        for uid in chunk_uids.iter() {
            result.extend(&self.get_tfidf_by_chunk_uid(*uid)?);
        }

        result.uid = Some(uid);
        Ok(result)
    }

    pub fn get_external_base(&self, index: &ExternalIndex) -> Result<&Index, Error> {
        for (i, ext) in self.external_index_info.iter().enumerate() {
            if ext.path == index.path {
                return Ok(self.external_indexes.get(i).unwrap());
            }
        }

        Err(Error::NoSuchExternalIndex { index: index.clone() })
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

    pub(crate) fn get_rel_path(root_dir: &Path, real_path: &Path) -> Result<Path, Error> {
        Ok(diff(
            &normalize(
                // in order to calc diff, it needs a full path
                &join(
                    &current_dir()?,
                    &real_path,
                )?,
            )?,
            &normalize(
                &join(
                    &current_dir()?,
                    &root_dir,
                )?,
            )?,
        )?)
    }

    // root_dir/.ragit/chunks/chunk_uid_prefix/chunk_uid_suffix.chunk
    fn get_chunk_path(root_dir: &Path, chunk_uid: Uid) -> Path {
        let chunks_at = join3(
            root_dir,
            &INDEX_DIR_NAME,
            &CHUNK_DIR_NAME,
        ).unwrap();
        let chunk_uid_prefix = chunk_uid.get_prefix();
        let chunk_uid_suffix = chunk_uid.get_suffix();

        join3(
            &chunks_at,
            &chunk_uid_prefix,
            &set_extension(
                &chunk_uid_suffix,
                "chunk",
            ).unwrap(),
        ).unwrap()
    }

    // root_dir/.ragit/file_index/file_uid_prefix/file_uid_suffix
    fn get_file_index_path(root_dir: &Path, file_uid: Uid) -> Path {
        let index_at = join3(
            root_dir,
            &INDEX_DIR_NAME,
            &FILE_INDEX_DIR_NAME,
        ).unwrap();
        let file_uid_prefix = file_uid.get_prefix();
        let file_uid_suffix = file_uid.get_suffix();

        join3(
            &index_at,
            &file_uid_prefix,
            &file_uid_suffix,
        ).unwrap()
    }

    fn get_image_path(root_dir: &str, image_key: &str, extension: &str) -> Path {
        normalize(
            &join4(
                root_dir,
                &INDEX_DIR_NAME.to_string(),
                &IMAGE_DIR_NAME.to_string(),
                &set_extension(image_key, extension).unwrap(),
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

    fn add_file_index(&mut self, file_uid: Uid, uids: &[Uid]) -> Result<(), Error> {
        let file_index_path = Index::get_file_index_path(&self.root_dir, file_uid);
        let parent_path = parent(&file_index_path)?;

        if !exists(&parent_path) {
            create_dir_all(&parent_path)?;
        }

        uid::save_to_file(&file_index_path, uids)
    }

    fn remove_file_index(&mut self, file_uid: Uid) -> Result<(), Error> {
        let file_index_path = Index::get_file_index_path(&self.root_dir, file_uid);

        if !exists(&file_index_path) {
            return Err(Error::NoSuchFile { path: None, uid: Some(file_uid) });
        }

        Ok(remove_file(&file_index_path)?)
    }

    pub fn get_chunks_of_file(&self, file_uid: Uid) -> Result<Vec<Uid>, Error> {
        for root_dir in std::iter::once(&self.root_dir).chain(
            self.external_indexes.iter().map(|index| &index.root_dir)
        ) {
            let file_index_path = Index::get_file_index_path(root_dir, file_uid);
            let mut result = vec![];

            if exists(&file_index_path) {
                for uid_str in read_string(&file_index_path)?.lines() {
                    result.push(uid_str.parse::<Uid>()?);
                }

                return Ok(result);
            }
        }

        return Err(Error::NoSuchFile { path: None, uid: Some(file_uid) });
    }

    fn count_external_chunks(&self) -> usize {
        self.external_indexes.iter().map(|index| index.chunk_count).sum()
    }

    pub fn load_image_by_key(&self, key: &str) -> Result<Vec<u8>, Error> {
        Ok(read_bytes(&Index::get_image_path(&self.root_dir, key, "png"))?)
    }
}
