use super::Chunk;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderableChunk {
    data: String,
    source: String,
}

impl RenderableChunk {
    pub fn fake(data: String, source: String) -> Self {
        RenderableChunk { data, source }
    }
}

impl From<Chunk> for RenderableChunk {
    fn from(c: Chunk) -> Self {
        RenderableChunk {
            source: c.render_source(),

            // TODO: render images
            // NOTE: `RenderableChunk.data` goes directly into pdl, so just insert
            //       `<|image_raw(png/...)|>`
            data: c.data,
        }
    }
}
