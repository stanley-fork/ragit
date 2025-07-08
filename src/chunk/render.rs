use super::{Chunk, MultiModalContent, into_multi_modal_contents};
use crate::error::Error;
use crate::index::Index;
use crate::uid::Uid;
use ragit_pdl::{encode_base64, escape_pdl_tokens};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RenderedChunk {
    // You can feed `pdl_data` directly to a pdl context.
    // Make sure to use a safe filter, like `{{chunk.pdl_data|safe}}`.
    pub pdl_data: String,

    // Human-readable data. It's used in `rag cat-file`, ragit-server's
    // file-content api, agent's file reader, and many more places.
    pub human_data: String,
    pub raw_data: Vec<MultiModalContent>,
    pub source: String,
}

impl RenderedChunk {
    pub fn fake(data: String, source: String) -> Self {
        RenderedChunk {
            pdl_data: data.to_string(),
            human_data: data.to_string(),
            raw_data: vec![MultiModalContent::Text { content: data }],
            source,
        }
    }
}

impl Chunk {
    pub fn render(self, index: &Index) -> Result<RenderedChunk, Error> {
        let human_data = self.data.clone();
        let raw_data = into_multi_modal_contents(&self.data, &self.images);
        let mut pdl_data = vec![];

        for c in raw_data.iter() {
            match c {
                MultiModalContent::Text { content } => {
                    pdl_data.push(escape_pdl_tokens(content));
                },
                MultiModalContent::Image { uid } => {
                    pdl_data.push(format!("<|raw_media(png:{})|>", encode_base64(&index.get_image_bytes_by_uid(uid.parse::<Uid>()?)?)));
                },
            }
        }

        Ok(RenderedChunk {
            human_data,
            pdl_data: pdl_data.join(""),
            raw_data,
            source: self.render_source(),
        })
    }
}
