use super::{CreateResponse, GetResponse, ModelKind};
use async_std::task;
use crate::{ApiProvider, Error};
use crate::record::{RecordAt, record_api_usage};
use crate::utils::download_file_from_url;
use json::JsonValue;
use ragit_fs::{
    WriteMode,
    write_bytes,
    write_log,
};
use std::time::Duration;

pub struct CreateRequest {
    pub model: ModelKind,

    // if it's missing, it tries to search ENV
    pub api_key: Option<String>,

    pub width: usize,
    pub height: usize,
    pub prompt: String,
    pub apply_watermark: bool,

    /// milliseconds
    pub timeout: Option<u64>,

    /// It tries 1 + max_retry times.
    pub max_retry: usize,

    /// milliseconds
    pub sleep_between_retries: u64,
}

impl CreateRequest {
    pub fn build_json_body(&self) -> JsonValue {
        let mut result = JsonValue::new_object();

        match self.model.get_api_provider() {
            ApiProvider::Replicate => {
                if self.model.uses_version_hash() {
                    result.insert("version", self.model.to_api_friendly_name()).unwrap();
                }

                let mut input = JsonValue::new_object();

                input.insert("width", self.width).unwrap();
                input.insert("height", self.height).unwrap();
                input.insert("prompt", self.prompt.clone()).unwrap();
                input.insert("apply_watermark", self.apply_watermark).unwrap();

                if self.model == ModelKind::Sdxl {
                    input.insert("refine", "base_image_refiner").unwrap();
                    input.insert("num_inference_steps", 50).unwrap();
                }

                result.insert("input", input).unwrap();
            },
            ApiProvider::OpenAi => {
                result.insert("model", self.model.to_api_friendly_name()).unwrap();
                result.insert("size", format!("{}x{}", self.width, self.height)).unwrap();
                result.insert("prompt", self.prompt.clone()).unwrap();

                if self.model == ModelKind::DallE3 {
                    result.insert("quality", "hd").unwrap();
                }
            },
            _ => unreachable!(),
        }

        result
    }

    /// It panics if its fields are not complete. If you're not sure, run `self.is_valid()` before sending a request.
    pub fn blocking_send(&self) -> Result<CreateResponse, Error> {
        futures::executor::block_on(self.send())
    }

    /// It panics if its fields are not complete. If you're not sure, run `self.is_valid()` before sending a request.
    pub async fn send(&self) -> Result<CreateResponse, Error> {
        let client = reqwest::Client::new();
        let mut curr_error = Error::NoTry;
        let api_key = self.get_api_key();

        let post_url = self.model.get_api_provider().get_image_create_api_url(self.model).unwrap();

        for _ in 0..(self.max_retry + 1) {
            let mut request = client.post(post_url.clone())
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .bearer_auth(api_key.clone())
                .body(self.build_json_body().dump());

            if let Some(t) = self.timeout {
                request = request.timeout(std::time::Duration::from_millis(t));
            }

            let response = request.send().await;

            match response {
                Ok(response) => match response.status().as_u16() {
                    200 => match response.text().await {
                        Ok(text) => match json::parse(&text) {
                            Ok(j) => {
                                let result = CreateResponse::from_json(&j, self.model.get_api_provider())?;
                                return Ok(result);
                            },
                            Err(e) => {
                                curr_error = Error::JsonError(e);
                            },
                        },
                        Err(e) => {
                            curr_error = Error::ReqwestError(e);
                        },
                    },
                    201 => match response.text().await {  // replicate returns 201 when successful
                        Ok(text) => match json::parse(&text) {
                            Ok(j) => {
                                let result = CreateResponse::from_json(&j, self.model.get_api_provider())?;
                                return Ok(result);
                            },
                            Err(e) => {
                                curr_error = Error::JsonError(e);
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

pub struct GetRequest {
    // if it's missing, it tries to search ENV
    pub api_key: Option<String>,
    pub model: ModelKind,  // it's required to record api usage
    pub handle_result: HandleResult,

    // replicate api uses `id` and openai api uses `url`
    pub id: String,
    pub url: Option<String>,

    /// milliseconds
    pub timeout: Option<u64>,

    /// It tries 1 + max_retry times.
    pub max_retry: usize,

    /// milliseconds
    pub sleep_between_retries: u64,
    pub record_api_usage_at: Option<RecordAt>,
}

#[derive(Clone)]
pub enum HandleResult {
    SaveTo(String),
    ReturnImageBytes,

    // only returns a boolean (whether the generation is complete or not)
    Nothing,
}

impl GetRequest {
    /// It panics if its fields are not complete. If you're not sure, run `self.is_valid()` before sending a request.
    pub fn blocking_send(&self) -> Result<GetResponse, Error> {
        futures::executor::block_on(self.send())
    }

    /// It panics if its fields are not complete. If you're not sure, run `self.is_valid()` before sending a request.
    pub async fn send(&self) -> Result<GetResponse, Error> {
        let client = reqwest::Client::new();
        let mut curr_error = Error::NoTry;
        let api_key = self.get_api_key();

        let url = self.model.get_api_provider().get_image_get_api_url(&self.id, &self.url).unwrap();

        if self.model.get_api_provider() == ApiProvider::OpenAi {
            let mut result = GetResponse {
                logs: String::new(),
                code: None,
                image_at: Some(url.clone()),
                image_bytes: None,
                complete: true,
                predict_time: None,
            };

            match &self.handle_result {
                HandleResult::Nothing => { /* nop */ },
                _ => {
                    let image_bytes = download_file_from_url(&url).await?;

                    if let HandleResult::SaveTo(path) = &self.handle_result {
                        write_bytes(
                            path,
                            &image_bytes,
                            WriteMode::CreateOrTruncate,
                        )?;
                    }

                    else {
                        result.image_bytes = Some(image_bytes);
                    }
                },
            }

            if let Some(key) = &self.record_api_usage_at {
                let (predict_time, dollars_per_1m_seconds) = match self.model.dollars_per_1m_image() {
                    Some(price) => (
                        1000,
                        price,
                    ),
                    None => (
                        (result.predict_time.unwrap_or(0.0) * 1_000.0) as u64,
                        self.model.dollars_per_1m_seconds(),
                    ),
                };

                if let Err(e) = record_api_usage(&key, 0, predict_time, 0, dollars_per_1m_seconds, false) {
                    write_log(
                        "record_api_usage",
                        &format!("record_api_usage({key:?}, ..) failed with {e:?}"),
                    );
                }
            }

            return Ok(result);
        }

        for _ in 0..(self.max_retry + 1) {
            let mut request = client.get(&url)
                .bearer_auth(api_key.clone());

            if let Some(t) = self.timeout {
                request = request.timeout(Duration::from_millis(t));
            }

            let response = request.send().await;

            match response {
                Ok(response) => match response.status().as_u16() {
                    200 => match response.text().await {
                        Ok(text) => match json::parse(&text) {
                            Ok(j) => {
                                let mut result = GetResponse::from_json(&j, ApiProvider::Replicate)?;

                                if result.complete {
                                    match &self.handle_result {
                                        HandleResult::Nothing => { /* nop */ },
                                        _ => {
                                            let image_bytes = download_file_from_url(result.image_at.as_ref().unwrap()).await?;

                                            if let HandleResult::SaveTo(path) = &self.handle_result {
                                                write_bytes(
                                                    path,
                                                    &image_bytes,
                                                    WriteMode::CreateOrTruncate,
                                                )?;
                                            }

                                            else {
                                                result.image_bytes = Some(image_bytes);
                                            }
                                        },
                                    }

                                    if let Some(key) = &self.record_api_usage_at {
                                        let (predict_time, dollars_per_1m_seconds) = match self.model.dollars_per_1m_image() {
                                            Some(price) => (
                                                1000,
                                                price,
                                            ),
                                            None => (
                                                (result.predict_time.unwrap_or(0.0) * 1_000.0) as u64,
                                                self.model.dollars_per_1m_seconds(),
                                            ),
                                        };

                                        if let Err(e) = record_api_usage(&key, 0, predict_time, 0, dollars_per_1m_seconds, false) {
                                            write_log(
                                                "record_api_usage",
                                                &format!("record_api_usage({key:?}, ..) failed with {e:?}"),
                                            );
                                        }
                                    }
                                }

                                return Ok(result);
                            },
                            Err(e) => {
                                curr_error = Error::JsonError(e);
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
