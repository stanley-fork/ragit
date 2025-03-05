//! Ragit uses json to interact with the world. This module defines json schema
//! of ragit objects. Some objects are valid json schema by themselves. For example,
//! a chunk is just a json file. There's no need to define a new schema for it. So
//! it's just a type alias: `type ChunkSchema = Chunk;`.
//! There's a `Prettify` trait, which makes ragit needlessly complex. Please read the
//! doc of the trait.

mod chunk;
mod file;
mod image;
mod model;
mod prettify;
mod query;

pub use chunk::ChunkSchema;
pub use file::FileSchema;
pub use image::ImageSchema;
pub use model::ModelSchema;
pub use prettify::Prettify;
pub use query::QueryResponseSchema;

pub(crate) use prettify::{prettify_timestamp, prettify_uid};
