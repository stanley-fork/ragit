use crate::chunk::{Chunk, RenderableChunk, merge_and_convert_chunks};
use crate::error::Error;
use crate::index::Index;
use crate::tree::generate_tree;
use ragit_api::Request;
use ragit_pdl::{
    Pdl,
    escape_pdl_tokens,
    parse_pdl,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::JoinSet;

pub mod config;
mod keyword;

pub use config::QueryConfig;
pub use keyword::Keywords;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryResponse {
    pub multi_turn_schema: Option<MultiTurnSchema>,
    pub retrieved_chunks: Vec<Chunk>,
    pub response: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryTurn {
    pub query: String,
    pub response: QueryResponse,
}

impl QueryTurn {
    pub fn new(query: String, response: QueryResponse) -> Self {
        QueryTurn { query, response }
    }
}

impl Index {
    /// It retrieves chunks that are related to `query`. If `super_rerank` is set, it calls `summaries_to_chunks` multiple times.
    /// That takes longer time, but is likely to have a better result.
    pub async fn retrieve_chunks(&self, query: &str, super_rerank: bool) -> Result<Vec<Chunk>, Error> {
        if !self.query_config.enable_rag || self.chunk_count == 0 {
            return Ok(vec![]);
        }

        let max_summaries = self.query_config.max_summaries;
        let max_retrieval = self.query_config.max_retrieval;
        let tfidf_limit = if super_rerank { max_summaries * 4 } else { max_summaries };
        let mut chunks = self.load_chunks_or_tfidf(query, tfidf_limit).await?;

        // Let's say `max_summaries` is 10 and `chunks.len()` is 40. That means the LLM can handle at most 10 chunks at a time,
        // but 40 chunks are given. So, it calls LLMs 4 times: the first call with the first 10 chunks, the next call with the next
        // 10 chunks, ... In each call, the LLM is asked to select at most 5 relevant chunks in the given 10 chunks.
        // Then it collects the chunks from the 4 LLM calls. If `chunks.len()` is still greater than 10, it does the same thing again.
        while chunks.len() > max_summaries {  // when `super_rerank` is set
            let mut t = generate_tree(chunks.len(), max_summaries);
            t.mark_range(&mut 0);
            let ranges = t.flatten_range();
            let each_chunk = ranges.iter().map(|(from, to)| chunks[*from..*to].to_vec()).collect::<Vec<Vec<Chunk>>>();
            let mut join_set = JoinSet::new();
            let mut new_chunks = vec![];

            for ec in each_chunk.iter() {
                let index = self.clone();
                let query = query.to_string();
                let ec = ec.to_vec();

                join_set.spawn(async move {
                    index.summaries_to_chunks(&query, ec, max_retrieval.max(max_summaries / 2)).await
                });
            }

            while let Some(res) = join_set.join_next().await {
                new_chunks.append(&mut res??);
            }

            chunks = new_chunks;
        }

        if chunks.len() > max_retrieval {
            chunks = self.summaries_to_chunks(
                query,
                chunks,
                max_retrieval,
            ).await?;
        }

        Ok(chunks)
    }

    /// A simple version of `query`, in case you're asking only a single question.
    pub async fn single_turn(
        &self,
        q: &str,
    ) -> Result<String, Error> {
        let result = self.query(q, vec![]).await?;
        Ok(result.response)
    }

    pub async fn query(
        &self,
        q: &str,
        history: Vec<QueryTurn>,
    ) -> Result<QueryResponse, Error> {
        // There's no need to rephrase the query if the rag pipeline is disabled.
        let (multi_turn_schema, rephrased_query) = if history.is_empty() || !self.query_config.enable_rag || self.chunk_count == 0 {
            (None, q.to_string())
        } else {
            let multi_turn_schema = self.rephrase_multi_turn(
                select_turns_for_context(&history, q),
            ).await?;
            let rephrased_query = if multi_turn_schema.is_query && multi_turn_schema.in_context {
                multi_turn_schema.query.clone()
            } else {
                q.to_string()
            };

            (Some(multi_turn_schema), rephrased_query)
        };
        let chunks = self.retrieve_chunks(&rephrased_query, self.query_config.super_rerank).await?;

        let response = if chunks.is_empty() {
            let mut history_turns = Vec::with_capacity(history.len() * 2);

            for h in history.iter() {
                history_turns.push(h.query.clone());
                history_turns.push(h.response.response.clone());
            }

            self.raw_request(
                q,
                history_turns,
            ).await?
        } else {
            self.answer_query_with_chunks(
                &rephrased_query,
                merge_and_convert_chunks(self, chunks.clone(), true /* render image */)?,
            ).await?
        };

        Ok(QueryResponse {
            multi_turn_schema,
            retrieved_chunks: chunks,
            response,
        })
    }

    pub async fn summaries_to_chunks(
        &self,
        query: &str,
        chunks: Vec<Chunk>,
        max_retrieval: usize,
    ) -> Result<Vec<Chunk>, Error> {
        let mut tera_context = tera::Context::new();
        tera_context.insert(
            "entries",
            &chunks.iter().enumerate().map(
                |(index, chunk)| format!(
                    "{}. {}\nsource: {}\nsummary: {}",
                    index + 1,
                    escape_pdl_tokens(&chunk.title),
                    escape_pdl_tokens(&chunk.render_source()),
                    escape_pdl_tokens(&chunk.summary),
                )
            ).collect::<Vec<_>>().join("\n\n"),
        );
        tera_context.insert(
            "query",
            &escape_pdl_tokens(&query),
        );
        tera_context.insert(
            "max_retrieval",
            &max_retrieval,
        );
        tera_context.insert(
            "max_index",
            &chunks.len(),
        );

        let Pdl { messages, schema } = parse_pdl(
            &self.get_prompt("rerank_summary")?,
            &tera_context,
            "/",  // TODO: `<|media|>` is not supported for this prompt
            true,
            true,
        )?;
        let request = Request {
            messages,
            frequency_penalty: None,
            max_tokens: None,
            temperature: None,
            timeout: self.api_config.timeout,
            max_retry: self.api_config.max_retry,
            sleep_between_retries: self.api_config.sleep_between_retries,
            dump_pdl_at: self.api_config.create_pdl_path(&self.root_dir, "rerank_summary"),
            dump_json_at: self.api_config.dump_log_at(&self.root_dir),
            model: self.get_model_by_name(&self.api_config.model)?,
            record_api_usage_at: self.api_config.dump_api_usage_at(&self.root_dir, "rerank_summary"),
            schema,
            schema_max_try: 3,
        };
        let chunk_indices = request.send_and_validate::<Vec<usize>>(vec![]).await?;

        Ok(chunks.into_iter().enumerate().filter(
            |(index, _)| chunk_indices.contains(&(index + 1))
        ).map(
            |(_, chunk)| chunk
        ).collect())
    }

    pub async fn answer_query_with_chunks(
        &self,
        query: &str,
        chunks: Vec<RenderableChunk>,
    ) -> Result<String, Error> {
        let mut tera_context = tera::Context::new();
        tera_context.insert(
            "chunks",
            &chunks,  // it's already escaped
        );
        tera_context.insert(
            "query",
            &escape_pdl_tokens(&query),
        );

        let Pdl { messages, .. } = parse_pdl(
            &self.get_prompt("answer_query")?,
            &tera_context,
            "/",  // TODO: `<|media|>` is not supported for this prompt
            true,
            true,
        )?;

        let request = Request {
            messages,
            timeout: self.api_config.timeout,
            max_retry: self.api_config.max_retry,
            sleep_between_retries: self.api_config.sleep_between_retries,
            dump_pdl_at: self.api_config.create_pdl_path(&self.root_dir, "answer_query_with_chunks"),
            dump_json_at: self.api_config.dump_log_at(&self.root_dir),
            model: self.get_model_by_name(&self.api_config.model)?,
            record_api_usage_at: self.api_config.dump_api_usage_at(&self.root_dir, "answer_query_with_chunks"),
            schema: None,
            ..Request::default()
        };

        let response = request.send().await?;

        Ok(response.get_message(0).unwrap().to_string())
    }

    pub async fn rephrase_multi_turn(
        &self,
        turns: Vec<String>,
    ) -> Result<MultiTurnSchema, Error> {
        let turns_json = Value::Array(turns.iter().map(|turn| Value::String(escape_pdl_tokens(turn))).collect());
        let turns_json = serde_json::to_string_pretty(&turns_json)?;
        let mut tera_context = tera::Context::new();
        tera_context.insert("turns", &turns_json);

        let Pdl { messages, schema } = parse_pdl(
            &self.get_prompt("multi_turn")?,
            &tera_context,
            "/",  // TODO: `<|media|>` is not supported for this prompt
            true,
            true,
        )?;

        let request = Request {
            messages,
            frequency_penalty: None,
            max_tokens: None,
            temperature: None,
            timeout: self.api_config.timeout,
            max_retry: self.api_config.max_retry,
            sleep_between_retries: self.api_config.sleep_between_retries,
            dump_pdl_at: self.api_config.create_pdl_path(&self.root_dir, "rephrase_multi_turn"),
            dump_json_at: self.api_config.dump_log_at(&self.root_dir),
            model: self.get_model_by_name(&self.api_config.model)?,
            record_api_usage_at: self.api_config.dump_api_usage_at(&self.root_dir, "rephrase_multi_turn"),
            schema,
            schema_max_try: 3,
        };

        Ok(request.send_and_validate::<MultiTurnSchema>(MultiTurnSchema::default()).await?)
    }

    pub async fn raw_request(
        &self,
        query: &str,
        history: Vec<String>,
    ) -> Result<String, Error> {
        let mut tera_context = tera::Context::new();
        tera_context.insert("query", &escape_pdl_tokens(&query));
        tera_context.insert("history", &history.iter().map(|h| escape_pdl_tokens(h)).collect::<Vec<_>>());

        let Pdl { messages, .. } = parse_pdl(
            &self.get_prompt("raw")?,
            &tera_context,
            "/",  // TODO: `<|media|>` is not supported for this prompt
            true,
            true,
        )?;
        let request = Request {
            messages,
            timeout: self.api_config.timeout,
            max_retry: self.api_config.max_retry,
            sleep_between_retries: self.api_config.sleep_between_retries,
            dump_pdl_at: self.api_config.create_pdl_path(&self.root_dir, "raw_request"),
            dump_json_at: self.api_config.dump_log_at(&self.root_dir),
            model: self.get_model_by_name(&self.api_config.model)?,
            record_api_usage_at: self.api_config.dump_api_usage_at(&self.root_dir, "raw_request"),
            schema: None,
            ..Request::default()
        };

        let response = request.send().await?;
        Ok(response.get_message(0).unwrap().to_string())
    }
}

/// Ragit supports multi-turn conversations. Since the pipeline
/// can handle only 1 query at a time, a multi-turn conversation
/// has to be rephrased into a single query. Ragit uses a
/// prompt to do that. This struct is a result of the prompt.
/// If `is_query` is set, the rephrasing is successful and you
/// can find the query in `query`. Otherwise, the last turn of
/// the conversation is not a query.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MultiTurnSchema {
    pub is_query: bool,
    pub in_context: bool,
    pub query: String,
}

impl Default for MultiTurnSchema {
    fn default() -> Self {
        MultiTurnSchema {
            is_query: false,
            in_context: false,
            query: String::new(),
        }
    }
}

fn select_turns_for_context(history: &[QueryTurn], query: &str) -> Vec<String> {
    match history.len() {
        0 => unreachable!(),
        1 => vec![
            history[0].query.to_string(),
            history[0].response.response.to_string(),
            query.to_string(),
        ],
        _ => {
            let last_turn = history.last().unwrap();

            match &last_turn.response.multi_turn_schema {
                None => vec![
                    last_turn.query.to_string(),
                    last_turn.response.response.to_string(),
                    query.to_string(),
                ],
                // use rephrased query if in-context
                Some(MultiTurnSchema {
                    is_query: true,
                    in_context: true,
                    query: rephrased_query,
                }) => vec![
                    rephrased_query.to_string(),
                    last_turn.response.response.to_string(),
                    query.to_string(),
                ],
                // still in context, but is not a query (e.g. greetings)
                Some(MultiTurnSchema {
                    is_query: false,
                    in_context: true,
                    query: _,
                }) => {
                    let before_last_turn = history.get(history.len() - 2).unwrap();

                    vec![
                        before_last_turn.query.to_string(),
                        before_last_turn.response.response.to_string(),
                        last_turn.query.to_string(),
                        last_turn.response.response.to_string(),
                        query.to_string(),
                    ]
                },
                // start a new context
                Some(MultiTurnSchema {
                    in_context: false,
                    ..
                }) => vec![
                    last_turn.query.to_string(),
                    last_turn.response.response.to_string(),
                    query.to_string(),
                ],
            }
        },
    }
}
