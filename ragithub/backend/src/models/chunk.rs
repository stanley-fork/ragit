use ragit::{Chunk, ChunkSource, MultiModalContent, into_multi_modal_contents};
use serde::{Deserialize, Serialize};

// `ragit::Chunk` is becoming more and more complicated and I don't want to
// expose too much internals of ragit to users. So I have decided to create
// another schema for chunk apis. I know fragmentation is bad, but I don't
// want to teach users how to get a chunk uid from 2 u128 integers.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChunkDetail {
    pub uid: String,
    pub data: Vec<MultiModalContent>,
    pub image_uids: Vec<String>,
    pub title: String,
    pub summary: String,
    pub file: Option<String>,
    pub file_index: Option<usize>,
    pub page_no: Option<usize>,
    pub timestamp: i64,
    pub model: String,
    pub ragit_version: String,
}

impl ChunkDetail {
    /// If you're building ragit-frontend in Rust, you're likely to depend on
    /// ragit-server and you're likely to want `ragit::Chunk::sortable_string`.
    /// This method tries to behave like `ragit::Chunk::sortable_string`. It it
    /// doesn't, please file an issue.
    pub fn sortable_string(&self) -> String {
        match (&self.file, self.file_index) {
            (Some(file), Some(file_index)) => format!("file: {file}-{file_index:09}"),
            _ => String::new(),
        }
    }
}

impl From<Chunk> for ChunkDetail {
    fn from(c: Chunk) -> ChunkDetail {
        let (file, file_index, page_no) = match &c.source {
            ChunkSource::File { path, index, page } => (Some(path.to_string()), Some(*index), page.clone()),
            // _ => (None, None, None),
        };

        ChunkDetail {
            uid: c.uid.to_string(),
            data: into_multi_modal_contents(&c.data, &c.images),
            image_uids: c.images.iter().map(|uid| uid.to_string()).collect(),
            title: c.title.clone(),
            summary: c.summary.clone(),
            file,
            file_index,
            page_no,
            timestamp: c.timestamp,
            model: c.build_info.model.clone(),
            ragit_version: c.build_info.ragit_version.clone(),
        }
    }
}
