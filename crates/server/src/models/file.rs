use ragit::MultiModalContent;
use serde::{Deserialize, Serialize};

// It can represent a file or a directory.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileDetail {
    pub r#type: FileType,
    pub content: Option<Vec<MultiModalContent>>,
    pub uid: Option<String>,
    pub path: String,
    pub chunks: Option<Vec<String>>,
    pub children: Option<Vec<FileSimple>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileSimple {
    pub r#type: FileType,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum FileType {
    File,
    Directory,
}
