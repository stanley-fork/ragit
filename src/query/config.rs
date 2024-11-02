use serde::{Deserialize, Serialize};

pub const QUERY_CONFIG_FILE_NAME: &str = "query.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryConfig {
    // if there are more than this amount of chunks, it runs tf-idf to select chunks
    pub max_titles: usize,

    // if there are more than this amount of chunks, it runs `rerank_title` prompt to select chunks
    pub max_summaries: usize,

    // if there are more than this amount of chunks, it runs `rerank_summary` prompt to select chunks
    pub max_retrieval: usize,
}

impl Default for QueryConfig {
    fn default() -> Self {
        QueryConfig {
            max_titles: 32,
            max_summaries: 10,
            max_retrieval: 3,
        }
    }
}
