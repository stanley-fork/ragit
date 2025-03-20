use serde::{Deserialize, Serialize};

// This struct is used for loading partial configurations from ~/.config/ragit/query.json
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct PartialQueryConfig {
    pub max_titles: Option<usize>,
    pub max_summaries: Option<usize>,
    pub max_retrieval: Option<usize>,
    pub enable_ii: Option<bool>,
    pub enable_rag: Option<bool>,
    pub super_rerank: Option<bool>,
}

impl PartialQueryConfig {
    // Apply partial config to a full config
    pub fn apply_to(&self, config: &mut QueryConfig) {
        if let Some(max_titles) = self.max_titles {
            config.max_titles = max_titles;
        }
        if let Some(max_summaries) = self.max_summaries {
            config.max_summaries = max_summaries;
        }
        if let Some(max_retrieval) = self.max_retrieval {
            config.max_retrieval = max_retrieval;
        }
        if let Some(enable_ii) = self.enable_ii {
            config.enable_ii = enable_ii;
        }
        if let Some(super_rerank) = self.super_rerank {
            config.super_rerank = super_rerank;
        }
    }
}

// Some fields are added after v0.1.1 and old config files might not have this field. So
// such field has to be decorated with `#[serde(default)]`. There's a small quirk
// with using `serde(default)`: `rag config --set super_rerank 0` is supposed to fail (a type error),
// but it does not fail and fallbacks to the default function.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct QueryConfig {
    /// This is deprecated and not used any more. It's here for backward compatibility.
    pub max_titles: usize,

    /// If there are more than this amount of chunks, it runs tf-idf to select chunks.
    pub max_summaries: usize,

    /// If there are more than this amount of chunks, it runs `rerank_summary` prompt to select chunks.
    pub max_retrieval: usize,

    /// If it's enabled, it uses an inverted index when running tf-idf search.
    /// It doesn't automatically build an inverted index when it's missing. You
    /// have to run `rag ii-build` manually to build the index.
    pub enable_ii: bool,

    /// You can disable the entire rag pipeline. If it's set, ragit never retrieves
    /// any chunk from the knowledge-base.
    #[serde(default = "_true")]
    pub enable_rag: bool,

    #[serde(default = "_false")]
    /// If it's enabled, it runs `rerank_summary.pdl` multiple times (usually 5 times) with much more candidates.
    /// It takes more time and money, but is likely to yield better result.
    pub super_rerank: bool,
}

fn _false() -> bool {
    false
}

fn _true() -> bool {
    true
}

impl Default for QueryConfig {
    fn default() -> Self {
        QueryConfig {
            max_titles: 32,
            max_summaries: 10,
            max_retrieval: 3,
            enable_ii: true,
            enable_rag: true,
            super_rerank: false,
        }
    }
}
