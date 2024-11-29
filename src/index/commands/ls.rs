use super::Index;
use crate::chunk::{self, Chunk};
use crate::error::Error;
use crate::uid::Uid;
use ragit_api::JsonType;
use ragit_fs::{
    file_name,
    file_size,
    parent,
    read_string,
    set_extension,
};
use serde::Serialize;
use serde_json::Value;

/// Convenient type for `ls-chunks`
#[derive(Clone, Debug, Serialize)]
pub struct LsChunk {
    pub title: String,
    pub summary: String,
    pub character_len: usize,
    pub file: String,
    pub index: usize,
    pub uid: Uid,
}

impl From<Chunk> for LsChunk {
    fn from(c: Chunk) -> LsChunk {
        LsChunk {
            title: c.title.clone(),
            summary: c.summary.clone(),
            character_len: c.data.chars().count(),
            file: c.file.clone(),
            index: c.index,
            uid: c.uid,
        }
    }
}

/// Convenient type for `ls-files`
#[derive(Clone, Debug, Serialize)]
pub struct LsFile {
    pub path: String,

    // if it's false, all the fields below have arbitrary values
    pub is_processed: bool,

    pub length: usize,
    pub uid: Uid,
    pub chunks: usize,
}

impl LsFile {
    pub fn dummy() -> Self {
        LsFile {
            path: String::new(),
            is_processed: false,
            length: 0,
            uid: Uid::dummy(),
            chunks: 0,
        }
    }
}

/// Convenient type for `ls-models`
#[derive(Clone, Debug, Serialize)]
pub struct LsModel {
    pub name: String,
    pub api_provider: String,
    pub api_key_env_var: Option<String>,
    pub can_read_images: bool,
    pub dollars_per_1b_input_tokens: u64,
    pub dollars_per_1b_output_tokens: u64,
    pub explanation: String,
}

/// Convenient type for `ls-images`
#[derive(Clone, Debug, Serialize)]
pub struct LsImage {
    pub uid: Uid,
    pub extracted_text: String,
    pub explanation: String,
    pub size: u64,  // bytes
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
    ) -> Result<Vec<LsChunk>, Error> where Filter: Fn(&LsChunk) -> bool, Map: Fn(LsChunk) -> LsChunk, Sort: Fn(&LsChunk) -> Key {
        let mut result = vec![];

        for chunk_file in self.get_all_chunk_files()? {
            let chunk = chunk::load_from_file(&chunk_file)?;
            let chunk: LsChunk = chunk.into();

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
    ) -> Result<Vec<LsFile>, Error> where Filter: Fn(&LsFile) -> bool, Map: Fn(LsFile) -> LsFile, Sort: Fn(&LsFile) -> Key {
        let mut result = vec![];

        for file in self.staged_files.iter() {
            result.push(LsFile {
                path: file.clone(),
                is_processed: false,
                ..LsFile::dummy()
            });
        }

        for (file, uid) in self.processed_files.iter() {
            let file_size = uid.get_data_size();
            result.push(LsFile {
                path: file.clone(),
                is_processed: true,
                length: file_size,
                uid: *uid,
                chunks: self.get_chunks_of_file(*uid).unwrap_or(vec![]).len(),
            });
        }

        result = result.into_iter().filter(filter).collect();
        result = result.into_iter().map(map).collect();
        result.sort_by_key(sort_key);

        Ok(result)
    }

    /// `rag ls-models`
    pub fn list_models<Filter, Map, Sort, Key: Ord>(
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Vec<LsModel> where Filter: Fn(&LsModel) -> bool, Map: Fn(LsModel) -> LsModel, Sort: Fn(&LsModel) -> Key {
        let mut result = vec![];

        for model in ragit_api::ChatModel::all_kinds() {
            let api_provider = model.get_api_provider();
            let ls_model = LsModel {
                name: model.to_human_friendly_name().to_string(),
                api_provider: api_provider.as_str().to_string(),
                api_key_env_var: api_provider.api_key_env_var().map(|v| v.to_string()),
                can_read_images: model.can_read_images(),
                dollars_per_1b_input_tokens: model.dollars_per_1b_input_tokens(),
                dollars_per_1b_output_tokens: model.dollars_per_1b_output_tokens(),
                explanation: model.explanation().to_string(),
            };

            if !filter(&ls_model) {
                continue;
            }

            let ls_model = map(ls_model);
            result.push(ls_model);
        }

        result.sort_by_key(sort_key);
        result
    }

    /// `rag ls-files`
    pub fn get_ls_file(&self, path: Option<String>, uid: Option<Uid>) -> Result<LsFile, Error> {
        if let Some(uid) = uid {
            for (path, uid_) in self.processed_files.iter() {
                if uid == *uid_ {
                    return Ok(self.get_ls_file_worker(path.to_string(), uid)?);
                }
            }
        }

        if let Some(path) = &path {
            if let Some(uid) = self.processed_files.get(path) {
                return Ok(self.get_ls_file_worker(path.to_string(), *uid)?);
            }

            if self.staged_files.contains(path) {
                return Ok(LsFile {
                    path: path.to_string(),
                    is_processed: false,
                    ..LsFile::dummy()
                })
            }
        }

        Err(Error::NoSuchFile { path, uid })
    }

    fn get_ls_file_worker(&self, path: String, uid: Uid) -> Result<LsFile, Error> {
        let file_size = uid.get_data_size();
        let chunks = self.get_chunks_of_file(uid).unwrap_or(vec![]).len();

        Ok(LsFile {
            path,
            is_processed: true,
            length: file_size,
            uid,
            chunks,
        })
    }

    /// `rag ls-images`
    pub fn list_images<Filter, Map, Sort, Key: Ord>(
        &self,
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Result<Vec<LsImage>, Error> where Filter: Fn(&LsImage) -> bool, Map: Fn(LsImage) -> LsImage, Sort: Fn(&LsImage) -> Key {
        let mut result = vec![];

        for image in self.get_all_image_files()? {
            let image = self.get_ls_image(Uid::from_prefix_and_suffix(
                &file_name(&parent(&image)?)?,
                &file_name(&image)?,
            )?)?;

            if !filter(&image) {
                continue;
            }

            result.push(map(image));
        }

        result.sort_by_key(sort_key);
        Ok(result)
    }

    /// `rag ls-images`
    pub fn get_ls_image(&self, uid: Uid) -> Result<LsImage, Error> {
        let description_path = Index::get_image_path(
            &self.root_dir,
            uid,
            "json",
        );
        let image_path = set_extension(&description_path, "png")?;
        let description = read_string(&description_path)?;
        let description = serde_json::from_str::<Value>(&description)?;

        match description {
            Value::Object(obj) => match (obj.get("extracted_text"), obj.get("explanation")) {
                (Some(extracted_text), Some(explanation)) => Ok(LsImage {
                    uid,
                    extracted_text: extracted_text.to_string(),
                    explanation: explanation.to_string(),
                    size: file_size(&image_path)?,
                }),
                _ => Err(Error::BrokenIndex(format!("`{description_path}` has a wrong schema."))),
            },
            _ => Err(Error::JsonTypeError {
                expected: JsonType::Object,
                got: (&description).into(),
            }),
        }
    }
}
