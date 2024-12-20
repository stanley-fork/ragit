use super::Chunk;
use crate::error::Error;
use crate::index::Index;
use ragit_pdl::{encode_base64, escape_pdl_tokens};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderableChunk {
    pub data: String,
    pub source: String,
}

impl RenderableChunk {
    pub fn fake(data: String, source: String) -> Self {
        RenderableChunk { data, source }
    }
}

impl Chunk {
    pub fn into_renderable(self, index: &Index) -> Result<RenderableChunk, Error> {
        let mut data = escape_pdl_tokens(&self.data);

        for image in self.images.iter() {
            data = data.replace(
                &format!("img_{image}"),
                &format!("<|raw_media(png:{})|>", encode_base64(&index.load_image_by_uid(*image)?)),
            );
        }

        Ok(RenderableChunk {
            source: self.render_source(),
            data,
        })
    }
}
