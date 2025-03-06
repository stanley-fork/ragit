use super::Chunk;
use crate::error::Error;
use crate::index::Index;
use ragit_pdl::{encode_base64, escape_pdl_tokens};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
    pub fn into_renderable(self, index: &Index, render_image: bool) -> Result<RenderableChunk, Error> {
        let mut data = escape_pdl_tokens(&self.data);

        if render_image {
            for image in self.images.iter() {
                data = data.replace(
                    &format!("img_{image}"),
                    &format!("<|raw_media(png:{})|>", encode_base64(&index.get_image_bytes_by_uid(*image)?)),
                );
            }
        }

        Ok(RenderableChunk {
            source: self.render_source(),
            data,
        })
    }
}
