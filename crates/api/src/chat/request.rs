use super::{ModelKind, Response};
use async_std::task;
use crate::{ApiProvider, Error, Message, Role};
use crate::record::{
    RecordAt,
    dump_pdl,
    record_api_usage,
};
use json::JsonValue;
use ragit_fs::write_log;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Request {
    pub messages: Vec<Message>,
    pub model: ModelKind,

    // if it's missing, it tries to search ENV vars
    pub api_key: Option<String>,
    pub temperature: Option<f64>,
    pub frequency_penalty: Option<f64>,

    pub max_tokens: Option<usize>,

    /// milliseconds
    pub timeout: Option<u64>,

    /// It tries 1 + max_retry times.
    pub max_retry: usize,

    /// milliseconds
    pub sleep_between_retries: u64,
    pub record_api_usage_at: Option<RecordAt>,

    /// It dumps the AI conversation in pdl format. See README file to know what pdl is.
    pub dump_pdl_at: Option<String>,
}

impl Request {
    pub fn is_valid(&self) -> bool {
        self.messages.len() > 1
        && self.messages.len() & 1 == 0  // the last message must be user's
        && self.messages[0].is_valid_system_prompt()  // I'm not sure whether all the models require the first message to be a system prompt. but it would be safer to guarantee that
        && {
            let mut flag = true;

            for (index, message) in self.messages[1..].iter().enumerate() {
                if index & 1 == 0 && !message.is_user_prompt() {
                    flag = false;
                    break;
                }

                else if index & 1 == 1 && !message.is_assistant_prompt() {
                    flag = false;
                    break;
                }
            }

            flag
        }
    }

    /// It panics if its fields are not complete. If you're not sure, run `self.is_valid()` before sending a request.
    pub fn build_json_body(&self) -> JsonValue {
        match self.model.get_api_provider() {
            ApiProvider::Groq | ApiProvider::OpenAi | ApiProvider::Cohere | ApiProvider::Ollama => {
                let mut result = JsonValue::new_object();
                result.insert("model", self.model.to_api_friendly_name()).unwrap();
                let mut messages = JsonValue::new_array();

                for message in self.messages.iter() {
                    messages.push(message.to_json(self.model.get_api_provider())).unwrap();
                }

                result.insert("messages", messages).unwrap();

                if let Some(temperature) = self.temperature {
                    result.insert("temperature", temperature).unwrap();
                }

                if let Some(frequency_penalty) = self.frequency_penalty {
                    result.insert("frequency_penalty", frequency_penalty).unwrap();
                }

                if let Some(max_tokens) = self.max_tokens {
                    result.insert("max_tokens", max_tokens).unwrap();
                }

                result
            },
            ApiProvider::Anthropic => {
                let mut result = JsonValue::new_object();
                result.insert("model", self.model.to_api_friendly_name()).unwrap();
                let mut messages = JsonValue::new_array();
                let mut system_prompt = vec![];

                for message in self.messages.iter() {
                    if message.role == Role::System {
                        system_prompt.push(message.content[0].unwrap_str());
                    }

                    else {
                        messages.push(message.to_json(ApiProvider::Anthropic)).unwrap();
                    }
                }

                let system_prompt = system_prompt.concat();

                if !system_prompt.is_empty() {
                    result.insert("system", system_prompt).unwrap();
                }

                result.insert("messages", messages).unwrap();

                if let Some(temperature) = self.temperature {
                    result.insert("temperature", temperature).unwrap();
                }

                if let Some(frequency_penalty) = self.frequency_penalty {
                    result.insert("frequency_penalty", frequency_penalty).unwrap();
                }

                // it's a required field
                result.insert("max_tokens", self.max_tokens.unwrap_or(2048)).unwrap();

                result
            },
            ApiProvider::Replicate => unreachable!(),  // for now, there's no chat model for replicate
            ApiProvider::Dummy => JsonValue::Null,
        }
    }

    /// NOTE: this function dies ocassionally, for no reason.
    /// It panics if its fields are not complete. If you're not sure, run `self.is_valid()` before sending a request.
    pub fn blocking_send(&self) -> Result<Response, Error> {
        futures::executor::block_on(self.send())
    }

    /// It panics if its fields are not complete. If you're not sure, run `self.is_valid()` before sending a request.
    pub async fn send(&self) -> Result<Response, Error> {
        if self.model.get_api_provider() == ApiProvider::Dummy {
            if let Some(path) = &self.dump_pdl_at {
                if let Err(e) = dump_pdl(
                    &self.messages,
                    "dummy",
                    path,
                    String::from("model: dummy, input_tokens: 0, output_tokens: 0, took: 0ms"),
                ) {
                    write_log(
                        "dump_pdl",
                        &format!("dump_pdl({path:?}, ..) failed with {e:?}"),
                    );

                    // TODO: should it return an error?
                    //       the api call was successful
                }
            }

            return Ok(Response::dummy(String::from("dummy")));
        }

        let started_at = Instant::now();
        let client = reqwest::Client::new();
        let mut curr_error = Error::NoTry;

        let post_url = self.model.get_api_provider().get_chat_api_url().unwrap();
        let body = self.build_json_body().dump();
        let api_key = self.get_api_key();
        write_log(
            "chat_request::send",
            &format!("entered chat_request::send() with {} bytes, model: {}", body.len(), self.model.to_human_friendly_name()),
        );

        for _ in 0..(self.max_retry + 1) {
            let mut request = client.post(post_url)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body.clone());

            if let ApiProvider::Anthropic = self.model.get_api_provider() {
                request = request.header("x-api-key", api_key.clone())
                    .header("anthropic-version", "2023-06-01");
            }

            else if !api_key.is_empty() {
                request = request.bearer_auth(api_key.clone());
            }

            if let Some(t) = self.timeout {
                request = request.timeout(Duration::from_millis(t));
            }

            write_log(
                "chat_request::send",
                "a request sent",
            );
            let response = request.send().await;
            write_log(
                "chat_request::send",
                "got a response from a request",
            );

            match response {
                Ok(response) => match response.status().as_u16() {
                    200 => match response.text().await {
                        Ok(text) => match Response::from_str(&text, self.model.get_api_provider()) {
                            Ok(result) => {

                                if let Some(key) = &self.record_api_usage_at {
                                    if let Err(e) = record_api_usage(
                                        key,
                                        result.get_prompt_token_count() as u64,
                                        result.get_output_token_count() as u64,
                                        self.model.dollars_per_1b_input_tokens(),
                                        self.model.dollars_per_1b_output_tokens(),
                                        false,
                                    ) {
                                        write_log(
                                            "record_api_usage",
                                            &format!("record_api_usage({key:?}, ..) failed with {e:?}"),
                                        );
                                    }
                                }

                                if let Some(path) = &self.dump_pdl_at {
                                    if let Err(e) = dump_pdl(
                                        &self.messages,
                                        &result.get_message(0).map(|m| m.to_string()).unwrap_or(String::new()),
                                        path,
                                        format!(
                                            "model: {}, input_tokens: {}, output_tokens: {}, took: {}ms",
                                            self.model.to_human_friendly_name(),
                                            result.get_prompt_token_count(),
                                            result.get_output_token_count(),
                                            Instant::now().duration_since(started_at.clone()).as_millis(),
                                        ),
                                    ) {
                                        write_log(
                                            "dump_pdl",
                                            &format!("dump_pdl({path:?}, ..) failed with {e:?}"),
                                        );

                                        // TODO: should it return an error?
                                        //       the api call was successful
                                    }
                                }

                                return Ok(result);
                            },
                            Err(e) => {
                                curr_error = e;
                            },
                        },
                        Err(e) => {
                            curr_error = Error::ReqwestError(e);
                        },
                    },
                    status_code => {
                        curr_error = Error::ServerError {
                            status_code,
                            body: response.text().await,
                        };

                        if let Some(path) = &self.dump_pdl_at {
                            if let Err(e) = dump_pdl(
                                &self.messages,
                                "",
                                path,
                                format!("{}# error: {curr_error:?} #{}", '{', '}'),
                            ) {
                                write_log(
                                    "dump_pdl",
                                    &format!("dump_pdl({path:?}, ..) failed with {e:?}"),
                                );
                            }
                        }
                    },
                },
                Err(e) => {
                    curr_error = Error::ReqwestError(e);
                },
            }

            task::sleep(Duration::from_millis(self.sleep_between_retries)).await
        }

        Err(curr_error)
    }

    fn get_api_key(&self) -> String {
        self.api_key.clone().unwrap_or_else(|| self.model.get_api_provider().get_api_key_from_env())
    }
}

impl Default for Request {
    fn default() -> Self {
        Request {
            messages: vec![],
            model: ModelKind::Llama70BGroq,
            api_key: None,

            temperature: None,
            frequency_penalty: None,
            max_tokens: None,
            timeout: Some(20_000),
            max_retry: 2,
            sleep_between_retries: 6_000,
            record_api_usage_at: None,
            dump_pdl_at: None,
        }
    }
}
