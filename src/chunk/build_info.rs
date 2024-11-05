use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct BuildInfo {
    file_reader_key: String,
    prompt_hash: String,
    model: String,
    ragit_version: String,
}

impl BuildInfo {
    pub fn dummy() -> Self {
        BuildInfo {
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
        BuildInfo {
            file_reader_key,
            prompt_hash,
            model,
            ragit_version: crate::VERSION.to_string(),
        }
    }
}
