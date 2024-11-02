mod api_config;
mod chunk;
mod error;
mod external;
mod index;
mod prompts;
mod query;

pub const INDEX_DIR_NAME: &str = ".rag_index";

pub use api_config::{ApiConfig, ApiConfigRaw};
pub use chunk::{
    Chunk,
    update_chunk_schema,
};
pub use error::Error;
pub use index::{
    AddMode,
    AddResult,
    BuildConfig,
    Index,
    ProcessedDoc,
    update_index_schema,
};
pub use query::{
    QueryConfig,
    Keywords,
    single_turn,
    multi_turn,
};

// My rules for version numbers
// Let's say I'm working on 0.1.2
//
// |                             | Cargo.toml  | this constant  |
// |-----------------------------|-------------|----------------|
// | working on 0.1.2            | 0.1.2       | "0.1.2-dev"    |
// | published version of 0.1.2  | 0.1.2       | "0.1.2"        |
// | after publishing 0.1.2      | 0.1.3       | "0.1.3-dev"    |
//
// Feel free to use whatever rules for your branches. But please keep version numbers
// distinguishable, so that chunks generated from your branches can easily be identified.
pub const VERSION: &str = "0.1.1-dev";
