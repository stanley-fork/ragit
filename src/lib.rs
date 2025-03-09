mod api_config;
pub mod chunk;
mod constant;
mod error;
mod index;
mod prompts;
mod query;
pub mod schema;
mod tree;
mod uid;

pub use api_config::ApiConfig;
pub use chunk::{
    Chunk,
    ChunkBuildInfo,
    ChunkSource,
    merge_and_convert_chunks,
};
pub use constant::*;
pub use error::Error;
pub use index::{
    AddMode,
    AddResult,
    Audit,
    BuildConfig,
    IIStatus,
    Index,
    LoadMode,
    MergeMode,
    MergeResult,
    ProcessedDoc,
    RecoverResult,
    RemoveResult,
    TfidfResult,
    VersionInfo,
    get_compatibility_warning,
};
pub use query::{
    Keywords,
    MultiTurnSchema,
    QueryConfig,
    QueryResponse,
    QueryTurn,
};
pub use uid::{Uid, UidQueryConfig, UidQueryResult};

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
pub const VERSION: &str = "0.3.3";
