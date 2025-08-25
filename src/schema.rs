//! Ragit uses json to interact with the world. This module defines json schema
//! of ragit objects. Some objects are valid json schema by themselves. For example,
//! a chunk is just a json file. There's no need to define a new schema for it. So
//! it's just a type alias: `type ChunkSchema = Chunk;`.
//! There's a `Prettify` trait, which makes ragit needlessly complex. Please read the
//! doc of the trait.
//!
//! Rules:
//! 1. I don't want people to read the files in `.ragit/`. I'll change their formats whenever
//!    I want, and ragit will take care of them.
//! 2. People do have to care about schemas that `--json` dumps. They're defined in this module
//!    and I'll try my best to keep them backward-compatible.

mod chunk;
mod file;
mod image;
mod model;
mod prettify;
mod query_response;
mod query_turn;

pub use chunk::ChunkSchema;
pub use file::FileSchema;
pub use image::ImageSchema;
pub use model::ModelSchema;
pub use prettify::Prettify;
pub use query_response::QueryResponseSchema;
pub use query_turn::QueryTurnSchema;

pub(crate) use prettify::{prettify_timestamp, prettify_uid};
