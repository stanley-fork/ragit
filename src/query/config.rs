use serde::{Deserialize, Serialize};

pub const QUERY_CONFIG_FILE_NAME: &str = "query.json";

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct QueryConfig {
    /// If there are more than this amount of chunks, it runs tf-idf to select chunks.
    pub max_titles: usize,

    /// If there are more than this amount of chunks, it runs `rerank_title` prompt to select chunks.
    pub max_summaries: usize,

    /// If there are more than this amount of chunks, it runs `rerank_summary` prompt to select chunks.
    pub max_retrieval: usize,

    /// If it's enabled, it uses an inverted index when running tf-idf search.
    /// It doesn't automatically build an inverted index when it's missing. You
    /// have to run `rag ii build` manually to build the index.
    pub enable_ii: bool,
}

impl Default for QueryConfig {
    fn default() -> Self {
        QueryConfig {
            max_titles: 32,
            max_summaries: 10,
            max_retrieval: 3,
            enable_ii: true,
        }
    }
}
