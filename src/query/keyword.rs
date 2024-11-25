use crate::ApiConfig;
use crate::index::tfidf::tokenize;
use ragit_api::{
    ChatRequest,
    Error,
    Message,
    MessageContent,
    RecordAt,
    Role,
    messages_from_pdl,
};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Keywords {
    // important keywords and less important keywords
    keywords: Vec<String>,
    extra: Vec<String>,

    // `keywords` are `weight` times more important than `extra`
    weight: usize,
}

impl Keywords {
    pub fn from_raw(keywords: Vec<String>) -> Self {
        Keywords {
            keywords,
            extra: vec![],
            weight: 1,
        }
    }

    pub fn with_weights(&self) -> Vec<(String, f32)> {
        self.keywords.iter().map(
            |keyword| (keyword.to_string(), self.weight as f32 / (self.weight + 1) as f32)
        ).chain(self.extra.iter().map(
            |extra| (extra.to_string(), 1.0 / (self.weight + 1) as f32)
        )).collect()
    }

    // keywords can be any string. it can be fed by user or ai
    // keywords are tokenized and deduplicated before tfidf
    pub fn tokenize(&self) -> HashMap<String, f32> {  // HashMap<Token, weight>
        let mut tokens = HashMap::new();

        for (keyword, weight) in self.with_weights() {
            for token in tokenize(&keyword) {
                match tokens.get(&token) {
                    Some(w) if *w > weight => {
                        // nop
                    },
                    _ => {
                        tokens.insert(token, weight);
                    },
                }
            }
        }

        tokens
    }
}

pub async fn extract_keywords(
    query: &str,
    api_config: &ApiConfig,
    pdl: &str,
) -> Result<Keywords, Error> {
    let mut dummy_context = tera::Context::new();
    dummy_context.insert("query", query);

    let messages = messages_from_pdl(
        pdl.to_string(),
        dummy_context,
    )?;

    let mut request = ChatRequest {
        api_key: api_config.api_key.clone(),
        messages,
        model: api_config.model,
        frequency_penalty: None,
        max_tokens: None,
        max_retry: api_config.max_retry,
        sleep_between_retries: api_config.sleep_between_retries,
        timeout: api_config.timeout,
        temperature: None,
        record_api_usage_at: api_config.dump_api_usage_at.clone().map(
            |path| RecordAt { path, id: String::from("extract_keywords") }
        ),
        dump_pdl_at: api_config.create_pdl_path("extract_keywords"),
    };
    let mut response = request.send().await?;
    let mut response_text = response.get_message(0).unwrap();
    let json_regex = Regex::new(r"(?s)[^{}]*(\{.*\})[^{}]*").unwrap();
    let mut mistakes = 0;

    let (keywords, extra) = loop {
        let mut error_message = String::new();

        if let Some(cap) = json_regex.captures(&response_text) {
            let json_text = cap[1].to_string();

            match serde_json::from_str::<Value>(&json_text) {
                Ok(j) => match j {
                    Value::Object(obj) if obj.len() == 2 => match (
                        obj.get("keywords"), obj.get("extra")
                    ) {
                        (Some(Value::Array(keywords)), Some(Value::Array(extra))) => {
                            let mut k = Vec::with_capacity(keywords.len());
                            let mut e = Vec::with_capacity(extra.len());
                            let mut has_error = false;

                            for keyword in keywords.iter() {
                                match keyword.as_str() {
                                    Some(s) => {
                                        k.push(s.to_string());
                                    },
                                    _ => {
                                        has_error = true;
                                        error_message = String::from("Make sure that all elements of \"keywords\" is a string.");
                                    },
                                }
                            }

                            for ex in extra.iter() {
                                match ex.as_str() {
                                    Some(s) => {
                                        e.push(s.to_string());
                                    },
                                    _ => {
                                        has_error = true;
                                        error_message = String::from("Make sure that all elements of \"keywords\" is a string.");
                                    },
                                }
                            }

                            if !has_error {
                                break (k, e);
                            }
                        },
                        (Some(_), Some(_)) => {
                            error_message = String::from("Make sure that \"keywords\" and \"extra\" are arrays of strings. Use an empty array instead of null or omitting fields.");
                        },
                        _ => {
                            error_message = String::from("Give me a json object with 2 keys: \"keywords\" and \"extra\".");
                        }, 
                    },
                    Value::Object(_) => {
                        error_message = String::from("Please give me a json object that contains 2 keys: \"keywords\" and \"extra\". Don't add keys to give extra information, put all your information in those two fields. Place an empty array if you have no keywords to extract.");
                    },
                    _ => {
                        error_message = String::from("Give me a json object with 2 keys: \"keywords\" and \"extra\". Don't omit fields.");
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

        // if a model is too stupid, it cannot create a valid json
        if mistakes > 5 {
            // it's the default search-keyword
            break (vec![query.to_string()], vec![]);
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

    Ok(Keywords {
        keywords,
        extra,
        weight: 4,  // configurable?
    })
}
