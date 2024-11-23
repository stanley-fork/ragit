use super::Index;
use crate::chunk::{self, Chunk};
use crate::error::Error;
use serde::Serialize;

#[derive(Serialize)]
pub struct RenderableFile {
    pub path: String,

    // if it's false, all the fields below have arbitrary values
    pub is_processed: bool,

    pub length: usize,
    pub uid: String,
    pub chunks: usize,
}

impl RenderableFile {
    pub fn dummy() -> Self {
        RenderableFile {
            path: String::new(),
            is_processed: false,
            length: 0,
            uid: String::new(),
            chunks: 0,
        }
    }
}

#[derive(Serialize)]
pub struct RenderableModel {
    pub name: String,
    pub api_provider: String,
    pub api_key_env_var: Option<String>,
    pub can_read_images: bool,
    pub dollars_per_1b_input_tokens: u64,
    pub dollars_per_1b_output_tokens: u64,
    pub explanation: String,
}

impl Index {
    /// `rag ls-chunks`
    ///
    /// It iterates all the chunks in the knowledge-base, which can be very expensive. If you know the uid of the chunk,
    /// use `get_chunk_by_uid` instead.
    pub fn list_chunks<Filter, Map, Sort, Key: Ord>(
        &self,
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Result<Vec<Chunk>, Error> where Filter: Fn(&Chunk) -> bool, Map: Fn(Chunk) -> Chunk, Sort: Fn(&Chunk) -> Key {
        let mut result = vec![];

        for chunk_file in self.get_all_chunk_files()? {
            let chunk = chunk::load_from_file(&chunk_file)?;

            if !filter(&chunk) {
                continue;
            }

            let chunk = map(chunk);
            result.push(chunk);
        }

        result.sort_by_key(sort_key);
        Ok(result)
    }

    /// `rag ls-files`
    pub fn list_files<Filter, Map, Sort, Key: Ord>(
        &self,
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Vec<RenderableFile> where Filter: Fn(&RenderableFile) -> bool, Map: Fn(RenderableFile) -> RenderableFile, Sort: Fn(&RenderableFile) -> Key {
        let mut result = vec![];

        for file in self.staged_files.iter() {
            result.push(RenderableFile {
                path: file.clone(),
                is_processed: false,
                ..RenderableFile::dummy()
            });
        }

        for (file, uid) in self.processed_files.iter() {
            let file_size = uid.get(55..).unwrap().parse::<usize>().unwrap();
            result.push(RenderableFile {
                path: file.clone(),
                is_processed: true,
                length: file_size,
                uid: uid.to_string(),
                chunks: self.get_chunks_of_file(uid).unwrap_or(vec![]).len(),
            });
        }

        result = result.into_iter().filter(filter).collect();
        result = result.into_iter().map(map).collect();
        result.sort_by_key(sort_key);

        result
    }

    /// `rag ls-models`
    pub fn list_models<Filter, Map, Sort, Key: Ord>(
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Vec<RenderableModel> where Filter: Fn(&RenderableModel) -> bool, Map: Fn(RenderableModel) -> RenderableModel, Sort: Fn(&RenderableModel) -> Key {
        let mut result = vec![];

        for model in ragit_api::ChatModel::all_kinds() {
            let api_provider = model.get_api_provider();
            let renderable = RenderableModel {
                name: model.to_human_friendly_name().to_string(),
                api_provider: api_provider.as_str().to_string(),
                api_key_env_var: api_provider.api_key_env_var().map(|v| v.to_string()),
                can_read_images: model.can_read_images(),
                dollars_per_1b_input_tokens: model.dollars_per_1b_input_tokens(),
                dollars_per_1b_output_tokens: model.dollars_per_1b_output_tokens(),
                explanation: model.explanation().to_string(),
            };

            if !filter(&renderable) {
                continue;
            }

            let renderable = map(renderable);
            result.push(renderable);
        }

        result.sort_by_key(sort_key);
        result
    }

    /// `rag ls-files`
    pub fn get_renderable_file(&self, path: Option<String>, uid: Option<String>) -> Result<RenderableFile, Error> {
        if let Some(path) = &path {
            if let Some(uid) = self.processed_files.get(path) {
                return Ok(self.get_renderable_file_worker(path.to_string(), uid.to_string())?);
            }
        }

        if let Some(uid) = &uid {
            for (path, uid_) in self.processed_files.iter() {
                if uid_ == uid {
                    return Ok(self.get_renderable_file_worker(path.to_string(), uid.to_string())?);
                }
            }
        }

        Err(Error::NoSuchFile { path, uid })
    }

    fn get_renderable_file_worker(&self, path: String, uid: String) -> Result<RenderableFile, Error> {
        let file_size = uid.get(55..).unwrap().parse::<usize>().unwrap();
        let chunks = self.get_chunks_of_file(&uid).unwrap_or(vec![]).len();

        Ok(RenderableFile {
            path,
            is_processed: true,
            length: file_size,
            uid,
            chunks,
        })
    }
}
