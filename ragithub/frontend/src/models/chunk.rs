use ragit::MultiModalContent;
pub use ragit_server::models::chunk::ChunkDetail;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct RenderableChunk {
    pub uid: String,
    pub data: Vec<MultiModalContent>,
    pub source: String,
}

impl From<ChunkDetail> for RenderableChunk {
    fn from(c: ChunkDetail) -> Self {
        let source = match (&c.file, c.file_index) {
            (Some(file), Some(file_index)) => format!(
                "{} chunk of `{file}`{}",
                match file_index {
                    0 => String::from("1st"),
                    1 => String::from("2nd"),
                    2 => String::from("3rd"),
                    n => format!("{}th", n + 1),
                },
                match c.page_no {
                    Some(page_no) => format!(" (page {page_no})"),
                    None => String::new(),
                },
            ),
            (Some(file), None) => file.to_string(),
            _ => String::new(),
        };

        RenderableChunk {
            uid: c.uid.clone(),
            data: c.data.clone(),
            source,
        }
    }
}
