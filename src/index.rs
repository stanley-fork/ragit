use crate::INDEX_DIR_NAME;
use crate::api_config::{ApiConfig, API_CONFIG_FILE_NAME, ApiConfigRaw};
use crate::chunk::{self, BuildInfo, Chunk, CHUNK_DIR_NAME, CHUNK_INDEX_DIR_NAME, Uid};
use crate::error::{Error, JsonType, get_type};
use crate::external::ExternalIndex;
use crate::prompts::{PROMPTS, PROMPT_DIR};
use crate::query::{Config as QueryConfig, Keywords, QUERY_CONFIG_FILE_NAME, extract_keywords};
use json::JsonValue;
use ragit_fs::{
    WriteMode,
    create_dir_all,
    diff,
    exists,
    is_dir,
    join,
    normalize,
    read_string,
    set_ext,
    write_bytes,
    write_string,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::collections::{HashMap, HashSet};
use std::iter;

mod commands;
mod config;
pub mod file;
pub mod tfidf;

pub use commands::{AddMode, AddResult};
pub use config::{Config, BUILD_CONFIG_FILE_NAME};
use file::{FileReader, get_file_hash};
use tfidf::{ProcessedDoc, TfIdfResult, TfIdfState, consume_tfidf_file};

pub const CONFIG_DIR_NAME: &str = "configs";
pub const INDEX_FILE_NAME: &str = "index.json";
pub const LOG_DIR_NAME: &str = "logs";

pub type Path = String;
pub type FileHash = String;

// all the `Path` are normalized relative paths
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub config: Config,
    #[serde(skip)]
    pub query_config: QueryConfig,
    #[serde(skip)]
    pub api_config: ApiConfig,
    #[serde(skip)]
    prompts: HashMap<String, String>,
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
            &CHUNK_INDEX_DIR_NAME.to_string(),
        ))?;

        let config = Config::default();
        let query_config = QueryConfig::default();
        let api_config = ApiConfig::default();
        let api_config_raw = ApiConfigRaw::default();

        let mut result = Index {
            ragit_version: crate::VERSION.to_string(),
            chunk_count: 0,
            staged_files: vec![],
            processed_files: HashMap::new(),
            curr_processing_file: None,
            config,
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
            &serde_json::to_vec_pretty(&result.config)?,
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
        read_only: bool,
    ) -> Result<Self, Error> {
        let index_json = read_string(&Index::get_rag_path(
            &root_dir,
            &INDEX_FILE_NAME.to_string(),
        ))?;

        let mut result = serde_json::from_str::<Index>(&index_json)?;
        result.root_dir = root_dir;
        result.config = serde_json::from_str::<Config>(
            &read_string(&result.get_build_config_path()?)?,
        )?;
        result.query_config = serde_json::from_str::<QueryConfig>(
            &read_string(&result.get_query_config_path()?)?,
        )?;
        result.api_config_raw = serde_json::from_str::<ApiConfigRaw>(
            &read_string(&result.get_api_config_path()?)?,
        )?;
        result.api_config = result.init_api_config(&result.api_config_raw)?;

        if result.ragit_version != crate::VERSION {
            // TODO
            // 1. show warning message
            // 2. impl auto-version-migration
        }

        if !read_only {
            result.remove_garbage_files()?;
            result.save_to_file()?;
        }

        result.load_prompts()?;
        result.load_external_indexes()?;
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
            // `load_or_init` cannot be done in read-only mode, because read-only `init` doesn't make sense
            Index::load(root_dir, false)
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

    fn chunk_files(&self) -> impl std::iter::Iterator<Item = &Path> {
        self.chunk_files.keys()
    }

    // Rust doesn't allow me to return `.keys().map()` as `impl Iter<Item = Path>`
    fn chunk_files_real_path(&self) -> Vec<Path> {
        self.chunk_files.keys().map(|chunk_file| Index::get_chunk_path(&self.root_dir, chunk_file)).collect()
    }

    fn tfidf_files(&self) -> Vec<Path> {
        self.chunk_files().map(|chunk| set_ext(chunk, "tfidf").unwrap()).collect()
    }

    fn tfidf_files_real_path(&self) -> Vec<Path> {
        self.tfidf_files().iter().map(|rel_path| Index::get_chunk_path(&self.root_dir, rel_path)).collect()
    }

    // garbage chunks: when an indexing is interrupted, chunks of unfinished file belongs to nowhere
    // in order to prevent creating the chunks again, it has to remove all those chunks
    fn remove_garbage_files(&mut self) -> Result<(), Error> {
        if let Some(file) = self.curr_processing_file.clone() {
            self.remove_file(file.clone())?;
            self.staged_files.push(file);
        }

        Ok(())
    }

    fn load_curr_processing_chunks(&self) -> Result<Vec<Chunk>, Error> {
        chunk::load_from_file(&Index::get_chunk_path(&self.root_dir, &self.get_curr_processing_chunks_path()))
    }

    fn get_curr_processing_chunks_path(&self) -> Path {
        for (path, count) in self.chunk_files.iter() {
            if *count < self.config.chunks_per_json {
                return path.to_string();
            }
        }

        unreachable!()
    }

    fn create_new_chunk_file(&mut self) -> Result<(), Error> {
        let rel_path = create_chunk_file_name();
        let real_path = Index::get_chunk_path(
            &self.root_dir,
            &rel_path,
        );
        chunk::save_to_file(
            &real_path,
            &[],
            self.config.compression_threshold,
            self.config.compression_level,
        )?;
        self.chunk_files.insert(rel_path, 0);

        Ok(())
    }

    pub async fn build_knowledge_base(&mut self, dashboard: bool) -> Result<(), Error> {
        let mut chunks = self.load_curr_processing_chunks()?;

        if !dashboard {
            // TODO: more flexible println, for example, respecting `--verbose` or `--quiet`
            println!("Starting index creation. Press Ctrl+C to pause the process. You can resume from where you left off (almost).\nRun `rag build --dashboard` for detailed information.");
        }

        else {
            self.render_dashboard()?;
        }

        let prompt = self.get_prompt("summarize")?;
        let mut hasher = Sha3_256::new();
        hasher.update(prompt.as_bytes());
        let prompt_hash = hasher.finalize();
        let prompt_hash = format!("{prompt_hash:064x}");

        while let Some(doc) = self.staged_files.pop() {
            // TODO: reject a file if it's inside `.rag_index`
            let real_path = Index::get_data_path(
                &self.root_dir,
                &doc,
            );

            let mut fd = FileReader::new(
                doc.clone(),
                real_path.clone(),
                self.config.clone(),
            )?;
            self.curr_processing_file = Some(doc.clone());
            let build_info = BuildInfo::new(
                fd.file_reader_key(),
                prompt_hash.clone(),
                self.api_config.model.to_human_friendly_name().to_string(),
            );

            while fd.can_generate_chunk() {
                if !dashboard {
                    println!(
                        "Creating index... staged files: {}, processed files: {}, processed chunks: {}",
                        self.staged_files.len() + if self.curr_processing_file.is_some() { 1 } else { 0 },
                        self.processed_files.len(),
                        self.chunk_count,
                    );
                }

                else {
                    self.render_dashboard()?;
                }

                let chunk_path = self.get_curr_processing_chunks_path();
                let new_chunk = fd.generate_chunk(&self.api_config, &prompt, build_info.clone()).await?;
                self.add_chunk_index(&new_chunk.uid, &chunk_path)?;
                chunks.push(new_chunk);
                self.chunk_count += 1;
                self.chunk_files.insert(chunk_path.clone(), chunks.len());

                chunk::save_to_file(
                    &Index::get_chunk_path(&self.root_dir, &chunk_path),
                    &chunks,
                    self.config.compression_threshold,
                    self.config.compression_level,
                )?;

                if chunks.len() >= self.config.chunks_per_json {
                    self.create_new_chunk_file()?;
                    chunks = self.load_curr_processing_chunks()?;
                }

                self.save_to_file()?;
            }

            self.processed_files.insert(doc.clone(), get_file_hash(&real_path)?);
            self.curr_processing_file = None;
            self.save_to_file()?;
        }

        Ok(())
    }

    pub fn run_tfidf(
        &self,
        keywords: Keywords,

        // 1. why not HashSet<Uid>?
        // 2. for now, no code does something with this value
        ignored_chunks: Vec<Uid>,
    ) -> Result<Vec<TfIdfResult<Uid>>, Error> {
        // TODO: tfidf on titles?
        let mut tfidf_data = TfIdfState::new(&keywords);
        let mut tfidf_summary = TfIdfState::new(&keywords);

        for tfidf_file in self.tfidf_files_real_path() {
            consume_tfidf_file(
                tfidf_file,
                &ignored_chunks,
                &mut tfidf_data,
                &mut tfidf_summary,
            )?;
        }

        for external_index in self.external_indexes.iter() {
            for tfidf_file in external_index.tfidf_files_real_path() {
                consume_tfidf_file(
                    tfidf_file,
                    &ignored_chunks,
                    &mut tfidf_data,
                    &mut tfidf_summary,
                )?;
            }
        }

        let mut top_docs = tfidf_data.get_top(self.query_config.max_summaries).into_iter().map(
            |(id, score)| TfIdfResult {
                id,
                score,
                category: String::from("data"),
            }
        ).collect::<Vec<_>>();

        // sometimes, tfidf_data wouldn't return any result
        // ex) when the raw data is not in English, or has lots of images
        // but the summaries are always in English, so you can always tfidf on summaries
        for doc in tfidf_summary.get_top(self.query_config.max_summaries) {
            if top_docs.len() >= self.query_config.max_summaries {
                break;
            }

            if !top_docs.iter().any(|TfIdfResult { id, .. }| id == &doc.0) {
                top_docs.push(TfIdfResult {
                    id: doc.0.clone(),
                    score: doc.1,
                    category: String::from("summary"),
                });
            }
        }

        Ok(top_docs)
    }

    // input and output has the same order
    // if any uid is missing, it returns Err
    pub fn get_chunks_by_uid(&self, uids: &[Uid]) -> Result<Vec<Chunk>, Error> {
        let mut visited_files = HashSet::new();
        let mut chunk_map = HashMap::with_capacity(uids.len());

        for uid in uids.iter() {
            let (root_dir, chunk_file) = self.get_chunk_file_by_index(uid)?;
            let chunk_file_real_path = Index::get_chunk_path(&root_dir, &chunk_file);

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

    // TODO: it's very stupid impl; must be rewritten
    pub fn get_tfidf_by_chunk_uid(
        &self,
        uid: Uid,

        // for now, each chunk has 2 types of ProcessedDoc: "data" and "summary"
        key: String,
    ) -> Result<ProcessedDoc, Error> {
        for tfidf_file in self.tfidf_files_real_path() {
            let tfidfs = tfidf::load_from_file(&tfidf_file)?;

            for processed_docs in tfidfs.iter() {
                match processed_docs.get(&key) {
                    Some(processed_doc) if processed_doc.chunk_uid.as_ref() == Some(&uid) => {
                        return Ok(processed_doc.clone());
                    },
                    _ => {},
                }
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
            &join(
                root_dir,
                &join(
                    &INDEX_DIR_NAME.to_string(),
                    rel_path,
                ).unwrap(),
            ).unwrap(),
        ).unwrap()
    }

    pub fn get_data_path(root_dir: &Path, rel_path: &Path) -> Path {
        normalize(
            &join(
                root_dir,
                rel_path,
            ).unwrap(),
        ).unwrap()
    }

    pub fn get_rel_path(root_dir: &Path, real_path: &Path) -> Path {
        normalize(
            &diff(
                real_path,
                root_dir,
            ).unwrap(),
        ).unwrap()
    }

    fn get_chunk_path(root_dir: &Path, chunk_name: &Path) -> Path {
        normalize(
            &join(
                root_dir,
                &join(
                    &INDEX_DIR_NAME.to_string(),
                    &join(
                        &CHUNK_DIR_NAME.to_string(),
                        chunk_name,
                    ).unwrap(),
                ).unwrap(),
            ).unwrap(),
        ).unwrap()
    }

    fn get_chunk_index_path(root_dir: &Path, chunk_uid: &Uid) -> Path {
        normalize(
            &join(
                root_dir,
                &join(
                    &INDEX_DIR_NAME.to_string(),
                    &join(
                        &CHUNK_INDEX_DIR_NAME.to_string(),
                        &format!("{}.json", chunk_uid.get(0..2).unwrap()),
                    ).unwrap(),
                ).unwrap(),
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

    pub fn init_api_config(&self, raw: &ApiConfigRaw) -> Result<ApiConfig, Error> {
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

    pub fn load_prompts(&mut self) -> Result<(), Error> {
        for prompt_name in PROMPTS.keys() {
            let prompt_path = Index::get_rag_path(
                &self.root_dir,
                &join(
                    PROMPT_DIR,
                    &set_ext(
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

    pub fn save_prompts(&self) -> Result<(), Error> {
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
                &set_ext(
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

    // NOTE: it's still naive that it iterates all the chunk_files.
    //       creating a map between files and chunks would be better,
    //       but I'm worried that would be a premature optimization
    pub fn remove_chunks_by_file_name(&mut self, file: Path) -> Result<(), Error> {
        let mut total_chunk_count = 0;

        // borrow checker issues
        let chunk_files: Vec<_> = self.chunk_files().map(
            |chunk| chunk.to_string()
        ).collect();

        for chunk_path in chunk_files.iter() {
            let real_path = Index::get_chunk_path(
                &self.root_dir,
                &chunk_path,
            );
            let mut chunks = chunk::load_from_file(&real_path)?;
            let mut chunks_to_remove = vec![];

            for chunk in chunks.iter() {
                if chunk.file == file {
                    chunks_to_remove.push(chunk.uid.clone());
                    self.remove_chunk_index(&chunk.uid)?;
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
                self.config.compression_threshold,
                self.config.compression_level,
            )?;
            self.chunk_files.insert(chunk_path.to_string(), chunks.len());
            total_chunk_count += chunks.len();
        }

        self.chunk_count = total_chunk_count;
        Ok(())
    }

    // It returns `(external_index's root_dir, rel_chunk_path)` because the chunk might be at an external knowledge-base
    pub fn get_chunk_file_by_index(&self, chunk_uid: &Uid) -> Result<(Path, Path), Error> {
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

    pub fn add_chunk_index(&self, chunk_uid: &Uid, chunk_file_rel_path: &Path) -> Result<(), Error> {
        let chunk_index_file = Index::get_chunk_index_path(&self.root_dir, chunk_uid);

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

        index.insert(chunk_uid, chunk_file_rel_path.to_string().into());
        write_string(
            &chunk_index_file,
            &JsonValue::Object(index).pretty(4),
            WriteMode::CreateOrTruncate,
        )?;

        Ok(())
    }

    // TODO: is this the best choice to return error when there's no chunk with the given uid?
    pub fn remove_chunk_index(&self, chunk_uid: &Uid) -> Result<(), Error> {
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

    fn count_external_chunks(&self) -> usize {
        self.external_indexes.iter().map(|index| index.chunk_count).sum()
    }
}

// it loads `.rag_llama_index.json`, modifies it and saves it
// it's useful when the schema of the index has changed
// make sure to backup files before running this!
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

// I want to make sure that each chunk file has a unique name. The ideal way is to hash the file, but I need more brainstorming for that.
fn create_chunk_file_name() -> String {
    format!("{:032x}.chunks", rand::random::<u128>())
}
