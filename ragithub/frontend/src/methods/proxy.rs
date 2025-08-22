use super::get_backend;
use crate::error::Error;
use crate::utils::into_query_string;
use ragit_fs::write_log;
use reqwest::{Client, RequestBuilder};
use reqwest::multipart::{Form, Part};
use serde_json::Value;
use std::collections::HashMap;
use warp::http::StatusCode;
use warp::reply::{Reply, json, with_header, with_status};

#[derive(Clone, Debug, Default)]
pub struct ProxyBuilder {
    pub method: Method,
    pub path: Vec<String>,
    pub query: HashMap<String, String>,
    pub api_key: Option<String>,
    pub body_raw: Option<Vec<u8>>,
    pub body_multiparts: Option<HashMap<String, Vec<u8>>>,
    pub response_type: ResponseType,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ResponseType {
    #[default]
    Json,
    String,
    Bytes,
    None,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Method {
    #[default]
    Get,
    Post,
}

impl ProxyBuilder {
    pub async fn send(&self) -> Box<dyn Reply> {
        let backend = get_backend();
        let url = format!(
            "{backend}/{}{}",
            self.path.join("/"),
            if self.query.is_empty() {
                String::new()
            } else {
                format!("?{}", into_query_string(&self.query))
            },
        );
        let request = match self.method {
            Method::Get => Client::new().get(&url),
            Method::Post => Client::new().post(&url),
        };

        match self.send_worker(request).await {
            Ok(r) => r,
            Err(e) => {
                write_log(
                    "ProxyBuilder::send()",
                    &format!("failed to send `{:?} {url}`: {e:?}", self.method),
                );
                return Box::new(with_status(
                    String::new(),
                    StatusCode::from_u16(500).unwrap(),
                ));
            },
        }
    }

    async fn send_worker(&self, mut request: RequestBuilder) -> Result<Box<dyn Reply>, Error> {
        if let Some(api_key) = &self.api_key {
            request = request.header("x-api-key", api_key);
        }

        if let Some(body_raw) = &self.body_raw {
            request = request.body(body_raw.to_vec());
        }

        if let Some(body_multiparts) = &self.body_multiparts {
            request = request.multipart(convert_form_data(body_multiparts)?);
        }

        let response = request.send().await?;

        if response.status() != 200 {
            Ok(Box::new(with_status(
                String::new(),

                // NOTE: `reqwest` and `warp` are looking at different versions of `http`
                StatusCode::from_u16(response.status().as_u16()).unwrap(),
            )))
        }

        else {
            match self.response_type {
                ResponseType::String => Ok(Box::new(with_header(
                    response.text().await?,
                    "Content-Type",
                    "text/plain; charset=utf-8",
                ))),
                ResponseType::Json => Ok(Box::new(json(&response.json::<Value>().await?))),
                ResponseType::Bytes => Ok(Box::new(with_header(
                    response.bytes().await?.to_vec(),
                    "Content-Type",
                    "application/octet-stream",
                ))),
                ResponseType::None => Ok(Box::new(with_status(
                    String::new(),
                    StatusCode::from_u16(200).unwrap(),
                ))),
            }
        }
    }
}

// `fetch_form_data(FormData) -> HashMap<_, _>` does not preserve the types of each
// field, so we have to guess them.
// If you try to send a valid utf-8 content with different content_type, it might
// behave oddly.
fn convert_form_data(form: &HashMap<String, Vec<u8>>) -> Result<Form, Error> {
    let mut result = Form::new();

    for (k, v) in form.iter() {
        if let Ok(s) = String::from_utf8(v.to_vec()) {
            result = result.text(k.to_string(), s);
        }

        else {
            result = result.part(k.to_string(), Part::bytes(v.to_vec()));
        }
    }

    Ok(result)
}
