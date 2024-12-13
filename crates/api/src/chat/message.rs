use crate::api_provider::ApiProvider;
use crate::error::Error;
use crate::json_type::JsonType;
use json::JsonValue;
use ragit_pdl::{
    ImageType,
    Message,
    MessageContent,
    Role,
    decode_base64,
    encode_base64,
};
use std::fmt;
use std::str::FromStr;

pub fn message_content_to_json(message: &MessageContent, api_provider: ApiProvider) -> JsonValue {
    match message {
        MessageContent::String(s) => {
            let mut content = JsonValue::new_object();
            content.insert("type", "text").unwrap();
            content.insert("text", s.to_string()).unwrap();

            content
        },
        MessageContent::Image { image_type, bytes } => match api_provider {
            ApiProvider::Anthropic => {
                let mut content = JsonValue::new_object();
                content.insert("type", "image").unwrap();

                let mut source = JsonValue::new_object();
                source.insert("type", "base64").unwrap();
                source.insert("media_type", image_type.get_media_type()).unwrap();
                source.insert("data", encode_base64(bytes)).unwrap();

                content.insert("source", source).unwrap();
                content
            },
            _ => {  // assume the others are all openai-compatible
                let mut content = JsonValue::new_object();
                content.insert("type", "image_url").unwrap();

                let mut image_url = JsonValue::new_object();
                image_url.insert("url", format!("data:{};base64,{}", image_type.get_media_type(), encode_base64(bytes))).unwrap();
                content.insert("image_url", image_url).unwrap();
                content
            },
        },
    }
}

pub fn message_content_from_json(j: &JsonValue) -> Result<Vec<MessageContent>, Error> {
    match j {
        j if j.as_str().is_some() => Ok(vec![MessageContent::String(j.as_str().unwrap().to_string())]),
        JsonValue::Array(contents) => {
            let mut result = Vec::with_capacity(contents.len());

            for content in contents.iter() {
                let element = message_content_from_json(content)?;

                if element.len() == 1 {
                    result.push(element[0].clone());
                }

                else {
                    return Err(Error::WrongSchema(String::from("You cannot nest MessageContent.")));
                }
            }

            Ok(result)
        },
        JsonValue::Object(content) => match content.get("type") {
            Some(s) if s.as_str() == Some("text") => match content.get("text") {
                Some(s) if s.is_string() => Ok(vec![MessageContent::String(s.to_string())]),
                Some(wrong_type) => Err(Error::JsonTypeError {
                    expected: JsonType::String,
                    got: wrong_type.into(),
                }),
                None => Err(Error::JsonObjectMissingField(String::from("text"))),
            },
            Some(s) if s.as_str() == Some("image") => match content.get("source") {
                Some(JsonValue::Object(source)) => match source.get("type") {
                    Some(s) if s.as_str() == Some("base64") => match source.get("data") {
                        Some(s) if s.is_string() => {
                            let bytes = decode_base64(s.as_str().unwrap())?;
                            let image_type = match source.get("media_type") {
                                Some(s) if s.is_string() => ImageType::from_media_type(s.as_str().unwrap())?,
                                Some(wrong_type) => {
                                    return Err(Error::JsonTypeError {
                                        expected: JsonType::String,
                                        got: wrong_type.into(),
                                    });
                                },
                                None => {
                                    return Err(Error::JsonObjectMissingField(String::from("media_type")));
                                },
                            };

                            Ok(vec![MessageContent::Image {
                                image_type,
                                bytes,
                            }])
                        },
                        Some(wrong_type) => Err(Error::JsonTypeError {
                            expected: JsonType::String,
                            got: wrong_type.into(),
                        }),
                        None => Err(Error::JsonObjectMissingField(String::from("data"))),
                    },
                    Some(wrong_type) => Err(Error::WrongSchema(format!("Available type is \"base64\", but got {wrong_type:?}"))),
                    None => Err(Error::JsonObjectMissingField(String::from("type"))),
                },
                Some(wrong_type) => Err(Error::JsonTypeError {
                    expected: JsonType::Object,
                    got: wrong_type.into(),
                }),
                None => Err(Error::JsonObjectMissingField(String::from("source"))),
            },
            Some(wrong_type) => Err(Error::WrongSchema(format!("Available types are \"text\" and \"image\", but got {wrong_type:?}"))),
            None => Err(Error::JsonObjectMissingField(String::from("type"))),
        },
        _ => Err(Error::JsonTypeError {
            expected: JsonType::Array,
            got: j.into(),
        }),
    }
}

pub fn message_contents_to_string(contents: &[MessageContent]) -> String {
    contents.iter().map(
        |content| content.to_string()
    ).collect::<Vec<String>>().concat()
}

pub fn message_contents_to_json_array(contents: &[MessageContent], api_provider: ApiProvider) -> JsonValue {
    JsonValue::Array(contents.iter().map(
        |content| message_content_to_json(content, api_provider)
    ).collect())
}

pub fn message_to_json(message: &Message, api_provider: ApiProvider) -> JsonValue {
    let mut result = JsonValue::new_object();
    result.insert("role", message.role.to_api_string()).unwrap();

    match (api_provider, message.content.len()) {
        (_, 0) => panic!("a message without any content"),
        (ApiProvider::Groq | ApiProvider::Anthropic | ApiProvider::Ollama, 1) if matches!(&message.content[0], MessageContent::String(_)) => match &message.content[0] {
            MessageContent::String(s) => {
                result.insert("content", s.clone()).unwrap();
            },
            MessageContent::Image { .. } => unreachable!(),
        },
        (ApiProvider::Groq | ApiProvider::Anthropic | ApiProvider::Cohere | ApiProvider::OpenAi | ApiProvider::Ollama, _) => {
            result.insert("content", message_contents_to_json_array(&message.content, api_provider)).unwrap();
        },
        (ApiProvider::Replicate, _) => panic!("no chat models for replicate"),
        (ApiProvider::Dummy, _) => unreachable!(),
    }

    result
}

pub fn message_from_json(j: &JsonValue) -> Result<Message, Error> {
    if let JsonValue::Object(j) = j {
        let role = match j.get("role") {
            Some(role) => if let Some(role) = role.as_str() {
                Role::from_str(role)?
            } else {
                return Err(Error::JsonTypeError {
                    expected: JsonType::String,
                    got: role.into(),
                })
            },
            None => {
                return Err(Error::JsonObjectMissingField(String::from("role")));
            },
        };
        let content = match j.get("content") {
            Some(content) => message_content_from_json(content)?,
            None => {
                return Err(Error::JsonObjectMissingField(String::from("content")));
            },
        };

        Ok(Message {
            role,
            content,
        })
    }

    else {
        Err(Error::JsonTypeError {
            expected: JsonType::Object,
            got: j.into(),
        })
    }
}
