use crate::{ApiProvider, Error};
use crate::json_type::{JsonType, get_type};
use json::JsonValue;

pub struct CreateResponse {
    pub id: String,
    pub status: String,
    pub image_url: Option<String>,
}

impl CreateResponse {
    pub fn from_json(j: &JsonValue, api_provider: ApiProvider) -> Result<Self, Error> {
        match api_provider {
            ApiProvider::Replicate => {
                let j = if let JsonValue::Object(obj) = j {
                    obj
                } else {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: get_type(j),
                    });
                };

                let id = match j.get("id") {
                    Some(id) => id.to_string(),
                    None => {
                        return Err(Error::JsonObjectMissingField(String::from("id")));
                    },
                };

                let status = match j.get("status") {
                    Some(status) => status.to_string(),
                    None => {
                        return Err(Error::JsonObjectMissingField(String::from("status")));
                    },
                };

                Ok(CreateResponse { id, status, image_url: None })
            },
            ApiProvider::OpenAi => match j {
                JsonValue::Object(obj) => match obj.get("data") {
                    Some(data) => match data {
                        JsonValue::Array(data) => match data.get(0) {
                            Some(JsonValue::Object(obj)) => match obj.get("url") {
                                Some(url) => match url.as_str() {
                                    Some(url) => Ok(CreateResponse {
                                        // every image must have a unique identifier
                                        id: format!("{:032x}", rand::random::<u128>()),
                                        status: String::new(),
                                        image_url: Some(url.to_string()),
                                    }),
                                    None => Err(Error::JsonTypeError {
                                        expected: JsonType::String,
                                        got: get_type(url),
                                    }),
                                },
                                None => Err(Error::JsonObjectMissingField(String::from("url"))),
                            },
                            Some(ty_err) => Err(Error::JsonTypeError {
                                expected: JsonType::Object,
                                got: get_type(ty_err),
                            }),
                            None => Err(Error::WrongSchema(String::from("no data found in response"))),
                        },
                        _ => Err(Error::JsonTypeError {
                            expected: JsonType::Array,
                            got: get_type(data),
                        }),
                    },
                    None => Err(Error::JsonObjectMissingField(String::from("data"))),
                },
                _ => Err(Error::JsonTypeError {
                    expected: JsonType::Object,
                    got: get_type(j),
                }),
            },
            _ => unreachable!(),
        }
    }
}

/// if `complete` is true, `image_at` must be `Some`
pub struct GetResponse {
    pub logs: String,
    pub code: Option<String>,  // `omost` returns this
    pub image_at: Option<String>,  // url
    pub image_bytes: Option<Vec<u8>>,
    pub complete: bool,
    pub predict_time: Option<f64>,  // seconds
}

impl GetResponse {
    pub fn from_json(j: &JsonValue, api_provider: ApiProvider) -> Result<Self, Error> {
        match j {
            JsonValue::Object(j) => {
                let logs = if let Some(logs) = j.get("logs") {
                    logs.to_string()
                } else {
                    return Err(Error::JsonObjectMissingField(String::from("logs")));
                };
                let status = j.get("status").map(|s| s.to_string()).unwrap_or_else(|| String::from("unknown"));
                let (code, image_at) = match j.get("output") {
                    Some(JsonValue::Object(output)) => {
                        let image_at = match output.get("image") {
                            Some(g) => match g.as_str() {
                                Some(s) => s.to_string(),
                                None => {
                                    return Err(Error::JsonTypeError {
                                        expected: JsonType::String,
                                        got: get_type(g),
                                    });
                                },
                            },
                            None => {
                                return Err(Error::JsonObjectMissingField(String::from("image")));
                            },
                        };

                        let code = output.get("code").map(|c| c.to_string());

                        (code, image_at)
                    },
                    Some(JsonValue::Array(output)) => match output.get(0) {
                        Some(image_at) => (None, image_at.to_string()),
                        None => {
                            return Err(Error::WrongSchema(String::from("get_response missing output")));
                        },
                    },
                    // some models directly return the url
                    Some(v) if v.as_str().is_some() => (None, v.as_str().unwrap().to_string()),
                    Some(v) => {
                        return Err(Error::JsonTypeError {
                            expected: JsonType::Object,
                            got: get_type(v),
                        });
                    },
                    None => {
                        return Err(Error::JsonObjectMissingField(String::from("output")));
                    },
                };
                let predict_time = if let Some(JsonValue::Object(metrics)) = j.get("metrics") {
                    if let Some(predict_time) = metrics.get("predict_time") {
                        predict_time.as_f64()
                    } else {
                        None
                    }
                } else {
                    None
                };

                Ok(GetResponse {
                    logs,
                    code,
                    image_at: Some(image_at),
                    image_bytes: None,
                    complete: status == "succeeded",
                    predict_time,
                })
            },
            _ => Err(Error::JsonTypeError {
                expected: JsonType::Object,
                got: get_type(j),
            }),
        }
    }
}
