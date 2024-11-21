use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ChunkBuildInfo {
    pub file_reader_key: String,
    pub prompt_hash: String,
    pub model: String,
    pub ragit_version: String,
}

impl ChunkBuildInfo {
    pub fn dummy() -> Self {
        ChunkBuildInfo {
            file_reader_key: String::new(),
            prompt_hash: String::new(),
            model: String::new(),
            ragit_version: String::new(),
        }
    }

    pub fn new(
        file_reader_key: String,
        prompt_hash: String,
        model: String,
    ) -> Self {
        ChunkBuildInfo {
            file_reader_key,
            prompt_hash,
            model,
            ragit_version: crate::VERSION.to_string(),
        }
    }
}
