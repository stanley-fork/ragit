use crate::ApiConfig;
use crate::chunk::{Chunk, RenderableChunk, merge_and_convert_chunks};
use crate::error::Error;
use crate::index::Index;
use ragit_api::{
    ChatRequest,
    JsonType,
    RecordAt,
};
use ragit_pdl::{
    Message,
    MessageContent,
    Pdl,
    Role,
    escape_pdl_tokens,
    parse_pdl,
};
use regex::Regex;
use serde_json::Value;

mod config;
mod keyword;

pub use config::{QueryConfig, QUERY_CONFIG_FILE_NAME};
pub use keyword::{Keywords, extract_keywords};

pub async fn retrieve_chunks(
    query: &str,
    index: &Index,
) -> Result<Vec<Chunk>, Error> {
    let mut chunks = index.load_chunks_or_tfidf(query).await?;

    if chunks.len() > index.query_config.max_summaries {
        chunks = titles_to_summaries(
            query,
            chunks.into_iter().map(|c| c.into()).collect(),
            &index.api_config,
            &index.get_prompt("rerank_title")?,
        ).await?;
    }

    if chunks.len() > index.query_config.max_retrieval {
        chunks = summaries_to_chunks(
            query,
            chunks.into_iter().map(|c| c.into()).collect(),
            index,
        ).await?;
    }

    Ok(chunks)
}

pub async fn single_turn(
    query: &str,
    index: &Index,
) -> Result<String, Error> {
    let chunks = retrieve_chunks(query, index).await?;

    if chunks.is_empty() {
        raw_request(
            query,
            &index.api_config,
            &index.get_prompt("raw")?,
        ).await
    }

    else {
        answer_query_with_chunks(
            query,
            merge_and_convert_chunks(index, chunks)?,
            &index.api_config,
            &index.get_prompt("answer_query")?,
        ).await
    }
}

pub async fn multi_turn(
    turns: Vec<String>,
    index: &Index,
) -> Result<String, Error> {
    single_turn(
        &rephrase_multi_turn(turns, &index.api_config, &index.get_prompt("multi_turn")?).await?,
        index,
    ).await
}

pub async fn titles_to_summaries(
    query: &str,
    chunks: Vec<Chunk>,
    api_config: &ApiConfig,
    pdl: &str,
) -> Result<Vec<Chunk>, Error> {
    let mut tera_context = tera::Context::new();
    tera_context.insert(
        "titles",
        &chunks.iter().enumerate().map(
            |(index, chunk)| format!("{}. {}", index + 1, escape_pdl_tokens(&chunk.title))
        ).collect::<Vec<_>>().join("\n"),
    );
    tera_context.insert(
        "query",
        &escape_pdl_tokens(&query),
    );
    tera_context.insert(
        "max_index",
        &chunks.len(),
    );

    // It's good to allow LLMs choose as many chunks as possible.
    // But allowing it to choose all the chunks might lead to an infinite loop.
    tera_context.insert(
        "max_retrieval",
        &(chunks.len() - 1),
    );

    let Pdl { messages, schema } = parse_pdl(
        pdl,
        &tera_context,
        true,
        true,
    )?;
    let request = ChatRequest {
        messages,
        frequency_penalty: None,
        max_tokens: None,
        temperature: None,
        timeout: api_config.timeout,
        max_retry: api_config.max_retry,
        sleep_between_retries: api_config.sleep_between_retries,
        api_key: api_config.api_key.clone(),
        dump_pdl_at: api_config.create_pdl_path("rerank_title"),
        dump_json_at: api_config.dump_log_at.clone(),
        model: api_config.model,
        record_api_usage_at: api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("rerank_title") }
        ),
        schema,
        schema_max_try: 3,
    };
    let title_indices = request.send_and_validate::<Vec<usize>>(vec![]).await?;

    Ok(chunks.into_iter().enumerate().filter(
        |(index, _)| title_indices.contains(&(index + 1))
    ).map(
        |(_, chunk)| chunk
    ).collect())
}

pub async fn summaries_to_chunks(
    query: &str,
    chunks: Vec<Chunk>,
    index: &Index,
) -> Result<Vec<Chunk>, Error> {
    let mut tera_context = tera::Context::new();
    tera_context.insert(
        "entries",
        &chunks.iter().enumerate().map(
            |(index, chunk)| format!(
                "{}. {}\nsource: {}\nsummary: {}",
                index + 1,
                escape_pdl_tokens(&chunk.title),
                escape_pdl_tokens(&chunk.file),
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
        &index.query_config.max_retrieval,
    );
    tera_context.insert(
        "max_index",
        &chunks.len(),
    );

    let Pdl { messages, schema } = parse_pdl(
        &index.get_prompt("rerank_summary")?,
        &tera_context,
        true,
        true,
    )?;
    let request = ChatRequest {
        messages,
        frequency_penalty: None,
        max_tokens: None,
        temperature: None,
        timeout: index.api_config.timeout,
        max_retry: index.api_config.max_retry,
        sleep_between_retries: index.api_config.sleep_between_retries,
        api_key: index.api_config.api_key.clone(),
        dump_pdl_at: index.api_config.create_pdl_path("rerank_summary"),
        dump_json_at: index.api_config.dump_log_at.clone(),
        model: index.api_config.model,
        record_api_usage_at: index.api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("rerank_summary") }
        ),
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
    query: &str,
    chunks: Vec<RenderableChunk>,
    api_config: &ApiConfig,
    pdl: &str,
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
        pdl,
        &tera_context,
        true,
        true,
    )?;

    let request = ChatRequest {
        messages,
        timeout: api_config.timeout,
        max_retry: api_config.max_retry,
        sleep_between_retries: api_config.sleep_between_retries,
        api_key: api_config.api_key.clone(),
        dump_pdl_at: api_config.create_pdl_path("answer_query_with_chunks"),
        dump_json_at: api_config.dump_log_at.clone(),
        model: api_config.model,
        record_api_usage_at: api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("answer_query_with_chunks") }
        ),
        schema: None,
        ..ChatRequest::default()
    };

    let response = request.send().await?;

    Ok(response.get_message(0).unwrap().to_string())
}

// TODO: What's wrong with this function? It seems like this function hasn't been tested for a while.
// 1. Use `request.send_and_validate`
// 2. Add test cases for this function.
pub async fn rephrase_multi_turn(
    turns: Vec<String>,
    api_config: &ApiConfig,
    pdl: &str,
) -> Result<String, Error> {
    let turns_json = Value::Array(turns.iter().map(|turn| Value::String(escape_pdl_tokens(turn))).collect());
    let turns_json = String::from_utf8_lossy(&serde_json::to_vec_pretty(&turns_json)?).to_string();
    let mut tera_context = tera::Context::new();
    tera_context.insert("turns", &turns_json);

    let Pdl { messages, schema } = parse_pdl(
        pdl,
        &tera_context,
        true,
        true,
    )?;

    let mut request = ChatRequest {
        messages,
        frequency_penalty: None,
        max_tokens: None,
        temperature: None,
        timeout: api_config.timeout,
        max_retry: api_config.max_retry,
        sleep_between_retries: api_config.sleep_between_retries,
        api_key: api_config.api_key.clone(),
        dump_pdl_at: api_config.create_pdl_path("rephrase_multi_turn"),
        dump_json_at: api_config.dump_log_at.clone(),
        model: api_config.model,
        record_api_usage_at: api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("rephrase_multi_turn") }
        ),
        schema,
        schema_max_try: 3,
    };

    let mut response = request.send().await?;
    let mut response_text = response.get_message(0).unwrap();
    let json_regex = Regex::new(r"(?s)[^{}]*(\{.*\})[^{}]*").unwrap();
    let mut mistakes = 0;

    let (is_query, query) = loop {
        let error_message;

        if let Some(cap) = json_regex.captures(&response_text) {
            let json_text = cap[1].to_string();

            match serde_json::from_str::<Value>(&json_text) {
                Ok(j) => match j {
                    Value::Object(obj) => match (
                        obj.get("is_query"),
                        obj.get("query"),
                    ) {
                        (Some(Value::Bool(false)), _) => {
                            break (false, String::new());
                        },
                        (Some(Value::Bool(true)), Some(query)) => match query.as_str() {
                            Some(s) => {
                                break (true, s.to_string());
                            },
                            None => {
                                error_message = format!("The value of \"query\" must be a string, not {}", format!("{:?}", JsonType::from(query)).to_ascii_lowercase());
                            },
                        },
                        (_, _) => {
                            error_message = String::from("Give me a json object with 3 keys: \"is_query\", \"in_context\" and \"query\". \"is_query\" and \"in_context\" are booleans and \"query\" is a string.");
                        },
                    },
                    _ => {
                        error_message = String::from("Give me a json object with 3 keys: \"is_query\", \"in_context\" and \"query\". \"is_query\" and \"in_context\" are booleans and \"query\" is a string.");
                    },
                },
                Err(_) => {
                    error_message = String::from("I cannot parse your output. It seems like your output is not a valid json. Please give me a valid json.");
                },
            }
        }

        else {
            error_message = String::from("I cannot find curly braces in your response. Please give me a valid json output.");
        }

        mistakes += 1;

        // if a model is too stupid, it cannot follow the instructions
        if mistakes > 5 {
            // default values
            break (false, String::new());
        }

        request.messages.push(Message {
            role: Role::Assistant,
            content: MessageContent::simple_message(response_text.to_string()),
        });
        request.messages.push(Message {
            role: Role::User,
            content: MessageContent::simple_message(error_message),
        });
        response = request.send().await?;
        response_text = response.get_message(0).unwrap();
    };

    if is_query {
        Ok(query)
    }

    else {
        Ok(turns.last().unwrap().to_string())
    }
}

pub async fn raw_request(
    query: &str,
    api_config: &ApiConfig,
    pdl: &str,
) -> Result<String, Error> {
    let mut tera_context = tera::Context::new();
    tera_context.insert("query", &escape_pdl_tokens(&query));

    let Pdl { messages, .. } = parse_pdl(
        pdl,
        &tera_context,
        true,
        true,
    )?;
    let request = ChatRequest {
        messages,
        timeout: api_config.timeout,
        max_retry: api_config.max_retry,
        sleep_between_retries: api_config.sleep_between_retries,
        api_key: api_config.api_key.clone(),
        dump_pdl_at: api_config.create_pdl_path("raw_request"),
        dump_json_at: api_config.dump_log_at.clone(),
        model: api_config.model,
        record_api_usage_at: api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("raw_request") }
        ),
        schema: None,
        ..ChatRequest::default()
    };

    let response = request.send().await?;
    Ok(response.get_message(0).unwrap().to_string())
}
