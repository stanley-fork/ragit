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
    Config as BuildConfig,
    Index,
    update_index_schema,
};
pub use query::{
    Config as QueryConfig,
    Keywords,
    single_turn,
    multi_turn,
};

pub const VERSION: &str = "0.0.0";
