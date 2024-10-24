use crate::ApiConfig;
use crate::chunk::{Chunk, RenderableChunk, Uid, merge_and_convert_chunks};
use crate::error::Error;
use crate::index::Index;
use json::JsonValue;
use ragit_api::{
    ChatRequest,
    Message,
    MessageContent,
    RecordAt,
    Role,
    get_type,
    messages_from_pdl,
};
use regex::Regex;

mod config;
mod keyword;

pub use config::{Config, QUERY_CONFIG_FILE_NAME};
pub use keyword::{Keywords, extract_keywords};

pub async fn retrieve_chunks(
    query: &str,
    index: &Index,
    ignored_chunks: Vec<Uid>,
) -> Result<Vec<Chunk>, Error> {
    let mut chunks = index.load_chunks_or_tfidf(
        query,
        ignored_chunks,
    ).await?;

    if chunks.len() > index.query_config.max_summaries {
        chunks = title_to_summaries(
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
    let chunks = retrieve_chunks(
        query,
        index,
        vec![],
    ).await?;

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

pub async fn title_to_summaries(
    query: &str,
    chunks: Vec<Chunk>,
    api_config: &ApiConfig,
    pdl: &str,
) -> Result<Vec<Chunk>, Error> {
    let mut tera_context = tera::Context::new();
    tera_context.insert(
        "titles",
        &chunks.iter().enumerate().map(
            |(index, chunk)| format!("{}. {}", index + 1, chunk.title)
        ).collect::<Vec<_>>().join("\n"),
    );
    tera_context.insert(
        "query",
        query,
    );

    let messages = messages_from_pdl(
        pdl.to_string(),
        tera_context,
    )?;

    let title_indices = get_array_of_numbers(
        messages,
        Some(1),

        // rust uses 0-index and llm uses 1-index
        Some(chunks.len() as i64),
        None,
        api_config,
        "rerank_title",
    ).await?;

    Ok(chunks.into_iter().enumerate().filter(
        |(index, _)| title_indices.contains(&((index + 1) as i64))
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
                chunk.title,
                chunk.render_source(),
                chunk.summary,
            )
        ).collect::<Vec<_>>().join("\n\n"),
    );
    tera_context.insert(
        "query",
        query,
    );
    tera_context.insert(
        "max_retrieval",
        &index.query_config.max_retrieval,
    );

    let messages = messages_from_pdl(
        index.get_prompt("rerank_summary")?,
        tera_context,
    )?;

    let chunk_indices = get_array_of_numbers(
        messages,
        Some(1),

        // rust uses 0-index and llm uses 1-index
        Some(chunks.len() as i64),
        Some(index.query_config.max_retrieval),
        &index.api_config,
        "rerank_summary",
    ).await?;

    Ok(chunks.into_iter().enumerate().filter(
        |(index, _)| chunk_indices.contains(&((index + 1) as i64))
    ).map(
        |(_, chunk)| chunk
    ).collect())
}

async fn get_array_of_numbers(
    messages: Vec<Message>,
    min_n: Option<i64>,
    max_n: Option<i64>,
    max_array_len: Option<usize>,
    api_config: &ApiConfig,
    log_title: &str,
) -> Result<Vec<i64>, Error> {
    let mut request = ChatRequest {
        messages: messages.clone(),
        frequency_penalty: None,
        max_tokens: None,
        temperature: None,
        timeout: api_config.timeout,
        max_retry: api_config.max_retry,
        sleep_between_retries: api_config.sleep_between_retries,
        api_key: api_config.api_key.clone(),
        dump_pdl_at: api_config.create_pdl_path(log_title),
        model: api_config.model,
        record_api_usage_at: api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from(log_title) }
        ),
    };

    let mut response = request.send().await?;
    let mut response_text = response.get_message(0).unwrap();
    let mut error_message = String::new();
    let mut mistakes = 0;

    let array_regex = Regex::new(r".*(\[\s*\d*(\s*\,\s*\d+)*\,?\s*\]).*").unwrap();
    let mut array_of_numbers;

    loop {
        array_of_numbers = vec![];

        if let Some(cap) = array_regex.captures(&response_text) {
            let json_text = cap[1].to_string();

            match json::parse(&json_text) {
                Ok(JsonValue::Array(numbers)) => {
                    let mut has_error = false;

                    for number in numbers.iter() {
                        match number.as_i64() {
                            Some(n) => {
                                array_of_numbers.push(n);

                                if let Some(m) = max_n {
                                    if n > m {
                                        has_error = true;
                                        error_message = format!("I see {n} in your array, which is not a valid number. The maximum number is {m}");
                                    }
                                }

                                if let Some(m) = min_n {
                                    if n < m {
                                        has_error = true;
                                        error_message = format!("I see {n} in your array, which is not a valid number. The minimum number is {m}");
                                    }
                                }
                            },
                            _ => {
                                has_error = true;
                                // TODO: for now, "which are the index of the titles." makes sense, but it would break something at someday
                                error_message = String::from("I cannot parse your array. Please make sure that the array only contains numbers, which are the index of the titles.");
                                break;
                            },
                        }
                    }

                    if let Some(len) = max_array_len {
                        if array_of_numbers.len() > len {
                            has_error = true;
                            error_message = format!("I need an array which has less than {} elements. Please pick the most important {} elements.", len + 1, len);
                        }
                    }

                    if !has_error {
                        break;
                    }
                },
                Ok(_) => unreachable!(),
                Err(_) => {
                    error_message = String::from("I cannot parse your array. Please give me the array in a valid json format.");
                },
            }
        } else {
            error_message = String::from("I cannot find a square bracket in your response. Please give me an array of numbers");
        }

        mistakes += 1;

        // if a model is too stupid, it cannot create a valid json
        if mistakes > 5 {
            // default value: select first ones
            return Ok((0..(messages.len().min(max_array_len.unwrap_or(usize::MAX)) as i64)).collect());
        }

        request.messages.push(Message {
            role: Role::Assistant,
            content: MessageContent::simple_message(response_text.to_string()),
        });
        request.messages.push(Message {
            role: Role::User,
            content: MessageContent::simple_message(error_message.clone()),
        });
        response = request.send().await?;
        response_text = response.get_message(0).unwrap();
    }

    Ok(array_of_numbers)
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
        &chunks,
    );
    tera_context.insert(
        "query",
        query,
    );

    let messages = messages_from_pdl(
        pdl.to_string(),
        tera_context,
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
        dump_pdl_at: api_config.create_pdl_path("answer_query_with_chunks"),
        model: api_config.model,
        record_api_usage_at: api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("answer_query_with_chunks") }
        ),
    };

    let response = request.send().await?;

    Ok(response.get_message(0).unwrap().to_string())
}

pub async fn rephrase_multi_turn(
    turns: Vec<String>,
    api_config: &ApiConfig,
    pdl: &str,
) -> Result<String, Error> {
    let turns_json = json::JsonValue::from(turns.clone()).pretty(2);
    let mut tera_context = tera::Context::new();
    tera_context.insert("turns", &turns_json);

    let messages = messages_from_pdl(
        pdl.to_string(),
        tera_context,
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
        model: api_config.model,
        record_api_usage_at: api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("rephrase_multi_turn") }
        ),
    };

    let mut response = request.send().await?;
    let mut response_text = response.get_message(0).unwrap();
    let json_regex = Regex::new(r"(?s)[^{}]*(\{.*\})[^{}]*").unwrap();
    let mut mistakes = 0;

    let (is_query, query) = loop {
        let error_message;

        if let Some(cap) = json_regex.captures(&response_text) {
            let json_text = cap[1].to_string();

            match json::parse(&json_text) {
                Ok(j) => match j {
                    JsonValue::Object(obj) => match (
                        obj.get("is_query"),
                        obj.get("query"),
                    ) {
                        (Some(JsonValue::Boolean(false)), _) => {
                            break (false, String::new());
                        },
                        (Some(JsonValue::Boolean(true)), Some(query)) => match query.as_str() {
                            Some(s) => {
                                break (true, s.to_string());
                            },
                            None => {
                                error_message = format!("The value of \"query\" must be a string, not {}", format!("{:?}", get_type(query)).to_ascii_lowercase());
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
    tera_context.insert("query", query);

    let messages = messages_from_pdl(
        pdl.to_string(),
        tera_context,
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
        dump_pdl_at: api_config.create_pdl_path("raw_request"),
        model: api_config.model,
        record_api_usage_at: api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("raw_request") }
        ),
    };

    let response = request.send().await?;
    Ok(response.get_message(0).unwrap().to_string())
}
