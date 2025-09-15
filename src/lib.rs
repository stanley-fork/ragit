use std::collections::HashMap;
use serde::{Deserialize, Serialize};

mod agent;
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

pub use agent::{
    ActionResult,
    ActionTrace,
    Action as AgentAction,
    AgentResponse,
};
pub use api_config::ApiConfig;
pub use chunk::{
    Chunk,
    ChunkBuildInfo,
    ChunkSource,
    MultiModalContent,
    into_multi_modal_contents,
    merge_and_convert_chunks,
};
pub use constant::*;
pub use error::Error;
pub use index::{
    AddMode,
    AddResult,
    Audit,
    BuildConfig,
    BuildResult,
    IIStatus,
    ImageDescription,
    Index,
    LoadMode,
    MergeMode,
    MergeResult,
    ProcessedDoc,
    PullResult,
    PushResult,
    RecoverResult,
    RemoveResult,
    Summary,
    SummaryMode,
    TfidfResult,
    UidOrStagedFile,
    VersionInfo,
    get_compatibility_warning,
};
pub use query::{
    Keywords,
    MultiTurnSchema,
    QueryConfig,
    QueryResponse,
    QueryTurn,
    RenderedQueryTurn,
    render_query_turns,
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
pub const VERSION: &str = "0.4.4-dev";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BuildOptions {
    pub version: String,
    pub profile: String,  // debug | release | production
    pub features: HashMap<String, bool>,
}

pub fn get_build_options() -> BuildOptions {
    let profile = if cfg!(feature = "production") {
        "production"
    } else if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    BuildOptions {
        version: VERSION.to_string(),
        profile: profile.to_string(),
        features: vec![
            (String::from("csv"), cfg!(feature = "csv")),
            (String::from("korean"), cfg!(feature = "korean")),
            (String::from("pdf"), cfg!(feature = "pdf")),
            (String::from("svg"), cfg!(feature = "svg")),
        ].into_iter().collect(),
    }
}
