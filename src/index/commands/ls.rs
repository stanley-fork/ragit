use super::Index;
use crate::chunk::{self, Chunk};
use crate::error::Error;
use serde::Serialize;

#[derive(Serialize)]
pub struct RenderableFile {
    pub name: String,

    // if it's false, all the fields below have arbitrary values
    pub is_processed: bool,

    pub length: usize,  // not in bytes
    pub hash: String,
}

#[derive(Serialize)]
pub struct RenderableModel {
    pub name: String,
    pub api_provider: String,
    pub api_key_env_var: Option<String>,
    pub explanation: String,
}

impl Index {
    pub fn list_chunks<Filter, Map, Sort, Key: Ord>(
        &self,
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Result<Vec<Chunk>, Error> where Filter: Fn(&Chunk) -> bool, Map: Fn(Chunk) -> Chunk, Sort: Fn(&Chunk) -> Key {
        let mut result = vec![];

        for chunk_file in self.chunk_files_real_path() {
            let chunks = chunk::load_from_file(&chunk_file)?;

            for chunk in chunks.into_iter() {
                if !filter(&chunk) {
                    continue;
                }

                let chunk = map(chunk);
                result.push(chunk);
            }
        }

        result.sort_by_key(sort_key);
        Ok(result)
    }

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
                name: file.clone(),
                is_processed: false,
                length: 0,
                hash: String::new(),
            });
        }

        for (file, hash) in self.processed_files.iter() {
            let file_size = hash.get(0..8).unwrap().parse::<usize>().unwrap();
            let file_hash = hash.get(9..).unwrap().to_string();
            result.push(RenderableFile {
                name: file.clone(),
                is_processed: true,
                length: file_size,
                hash: file_hash,
            });
        }

        result = result.into_iter().filter(filter).collect();
        result = result.into_iter().map(map).collect();
        result.sort_by_key(sort_key);

        result
    }

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
}
