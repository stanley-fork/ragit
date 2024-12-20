mod api_config;
mod chunk;
mod error;
mod index;
mod prompts;
mod query;
mod uid;

pub const INDEX_DIR_NAME: &str = ".ragit";

pub use api_config::{ApiConfig, ApiConfigRaw};
pub use chunk::{
    Chunk,
    ChunkBuildInfo,
    ChunkSource,
    merge_and_convert_chunks,
};
pub use error::Error;
pub use index::{
    AddMode,
    AddResult,
    BuildConfig,
    CloneResult,
    IIStatus,
    Index,
    LoadMode,
    LsChunk,
    LsFile,
    LsImage,
    LsModel,
    MergeMode,
    MergeResult,
    ProcessedDoc,
    RecoverResult,
    TfidfResult,
    VersionInfo,
    get_compatibility_warning,
};
pub use query::{
    QueryConfig,
    Keywords,
    single_turn,
    multi_turn,
};
pub use uid::{Uid, UidQuery, UidQueryResult};

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
pub const VERSION: &str = "0.2.0-dev";
