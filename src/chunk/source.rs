use crate::uid::Uid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum ChunkSource {
    /// Built directly from a file.
    /// It's `index`th chunk of `path`.
    /// `path` is a relative path.
    File { path: String, index: usize },

    /// Summary of multiple chunks.
    Chunks(Vec<Uid>),
}

impl ChunkSource {
    // this value is directly used to hash this instance
    pub fn hash_str(&self) -> String {
        match self {
            ChunkSource::File { path, index } => format!("{path}{index}"),
            ChunkSource::Chunks(chunk_uids) => {
                let mut result = Uid::dummy();

                for chunk_uid in chunk_uids.iter() {
                    result ^= *chunk_uid;
                }

                result.to_string()
            },
        }
    }

    pub fn set_path(&mut self, new_path: String) {
        match self {
            ChunkSource::File { path, .. } => { *path = new_path; },
            _ => panic!(),
        }
    }

    pub fn sortable_string(&self) -> String {
        match self {
            ChunkSource::File { path, index } => format!("file: {path}-{index:09}"),
            ChunkSource::Chunks(_) => format!("chunks: {}", self.hash_str()),
        }
    }
}
