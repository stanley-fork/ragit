use super::{Prettify, prettify_timestamp, prettify_uid};
use crate::error::Error;
use crate::index::Index;
use crate::uid::Uid;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
pub struct FileSchema {
    pub path: String,

    // if it's false, all the fields below have arbitrary values
    pub is_processed: bool,

    pub length: usize,
    pub uid: Uid,
    pub chunks: usize,

    // model of the most recent chunk
    pub model: String,

    // time stamp of the most recent chunk
    pub last_updated: i64,
}

impl FileSchema {
    pub fn dummy() -> Self {
        FileSchema {
            path: String::new(),
            is_processed: false,
            length: 0,
            uid: Uid::dummy(),
            chunks: 0,
            model: String::new(),
            last_updated: 0,
        }
    }
}

impl Index {
    pub fn get_file_schema(&self, path: Option<String>, uid: Option<Uid>) -> Result<FileSchema, Error> {
        if let Some(uid) = uid {
            for (path, uid_) in self.processed_files.iter() {
                if uid == *uid_ {
                    return Ok(self.get_file_schema_worker(path.to_string(), uid)?);
                }
            }
        }

        if let Some(path) = &path {
            if let Some(uid) = self.processed_files.get(path) {
                return Ok(self.get_file_schema_worker(path.to_string(), *uid)?);
            }

            if self.staged_files.contains(path) {
                return Ok(FileSchema {
                    path: path.to_string(),
                    is_processed: false,
                    ..FileSchema::dummy()
                })
            }
        }

        Err(Error::NoSuchFile { path, uid })
    }

    pub(crate) fn get_file_schema_worker(&self, path: String, uid: Uid) -> Result<FileSchema, Error> {
        let file_size = uid.get_data_size();
        let chunk_uids = self.get_chunks_of_file(uid).unwrap_or(vec![]);
        let mut chunks = Vec::with_capacity(chunk_uids.len());

        for chunk_uid in chunk_uids.iter() {
            chunks.push(self.get_chunk_by_uid(*chunk_uid)?);
        }

        chunks.sort_by_key(|chunk| chunk.timestamp);

        let (model, last_updated) = match chunks.last() {
            Some(chunk) => (chunk.build_info.model.clone(), chunk.timestamp),
            None => (String::new(), 0),
        };

        Ok(FileSchema {
            path,
            is_processed: true,
            length: file_size,
            uid,
            chunks: chunks.len(),
            model,
            last_updated,
        })
    }
}

impl Prettify for FileSchema {
    fn prettify(&self) -> Result<Value, Error> {
        let mut result = serde_json::to_value(self)?;

        if self.is_processed {
            if let Value::Object(obj) = &mut result {
                match obj.get_mut("uid") {
                    Some(uid) => { *uid = prettify_uid(uid) },
                    None => {},
                }

                match obj.get_mut("last_updated") {
                    Some(timestamp) => { *timestamp = prettify_timestamp(timestamp); },
                    None => {},
                }
            }
        }

        else {
            if let Value::Object(obj) = &mut result {
                for key in obj.keys().map(|k| k.to_string()).collect::<Vec<_>>() {
                    if key != "path" && key != "is_processed" {
                        obj.remove(&key);
                    }
                }
            }
        }

        Ok(result)
    }
}
