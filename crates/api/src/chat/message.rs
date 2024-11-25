use super::Role;
use crate::api_provider::ApiProvider;
use crate::error::Error;
use crate::json_type::JsonType;
use crate::utils::{decode_base64, encode_base64};
use json::JsonValue;
use ragit_fs::read_string;
use regex::Regex;
use std::fmt;
use std::str::FromStr;

mod media;

pub use media::{ImageType, MediaMessageBuilder};

/// Use `MediaMessageBuilder`, instead of constructing `MessageContent` from scratch
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MessageContent {
    String(String),
    Image {
        image_type: ImageType,
        bytes: Vec<u8>,
    },
}

impl MessageContent {
    pub fn unwrap_str(&self) -> &str {
        match self {
            MessageContent::String(s) => s.as_str(),
            _ => panic!("{self:?} is not a string"),
        }
    }

    pub fn to_json(&self, api_provider: ApiProvider) -> JsonValue {
        match self {
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

    pub fn from_json(j: &JsonValue) -> Result<Vec<Self>, Error> {
        match j {
            j if j.as_str().is_some() => Ok(vec![MessageContent::String(j.as_str().unwrap().to_string())]),
            JsonValue::Array(contents) => {
                let mut result = Vec::with_capacity(contents.len());

                for content in contents.iter() {
                    let element = MessageContent::from_json(content)?;

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

    pub fn simple_message(s: String) -> Vec<Self> {
        vec![MessageContent::String(s)]
    }
}

impl fmt::Display for MessageContent {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            MessageContent::String(s) => write!(fmt, "{s}"),
            MessageContent::Image { image_type, bytes } => write!(
                fmt,
                "<|raw_media({}:{})|>",
                image_type.to_extension(),
                encode_base64(bytes),
            ),
        }
    }
}

pub fn message_contents_to_string(contents: &[MessageContent]) -> String {
    contents.iter().map(
        |content| content.to_string()
    ).collect::<Vec<String>>().join("\n")
}

pub fn message_contents_to_json_array(contents: &[MessageContent], api_provider: ApiProvider) -> JsonValue {
    JsonValue::Array(contents.iter().map(
        |content| content.to_json(api_provider)
    ).collect())
}

/// Use `MediaMessageBuilder` to convert image/document files to a `Message` instance
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Message {
    pub role: Role,

    /// It's `Vec<MessageContent>` because a single message may contain multiple contents.
    /// There's no point in `[String(s), String(s), ...]` because the string contents are concatonated anyway.
    /// Multiple contents makes sense when there are images, like `[Image { .. }, Image { .. }]` or `[Image { .. }, String(s), ...]`.
    ///
    /// `Message` instances constructed from API response are guaranteed to
    ///
    /// 1. have exactly one content
    /// 2. the content is MessageContent::String
    pub content: Vec<MessageContent>,
}

impl Message {
    pub fn to_json(&self, api_provider: ApiProvider) -> JsonValue {
        let mut result = JsonValue::new_object();
        result.insert("role", self.role.to_api_string(api_provider)).unwrap();

        match (api_provider, self.content.len()) {
            (_, 0) => panic!("a message without any content"),
            (ApiProvider::Groq | ApiProvider::Anthropic | ApiProvider::Ollama, 1) if matches!(&self.content[0], MessageContent::String(_)) => match &self.content[0] {
                MessageContent::String(s) => {
                    result.insert("content", s.clone()).unwrap();
                },
                MessageContent::Image { .. } => unreachable!(),
            },
            (ApiProvider::Groq | ApiProvider::Anthropic | ApiProvider::Cohere | ApiProvider::OpenAi | ApiProvider::Ollama, _) => {
                result.insert("content", message_contents_to_json_array(&self.content, api_provider)).unwrap();
            },
            (ApiProvider::Replicate, _) => panic!("no chat models for replicate"),
            (ApiProvider::Dummy, _) => unreachable!(),
        }

        result
    }

    pub fn from_json(j: &JsonValue) -> Result<Self, Error> {
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
                Some(content) => MessageContent::from_json(content)?,
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

    pub fn simple_message(role: Role, message: String) -> Self {
        Message {
            role,
            content: vec![MessageContent::String(message)],
        }
    }

    pub fn is_valid_system_prompt(&self) -> bool {
        self.role == Role::System
        && self.content.len() == 1
        && matches!(&self.content[0], MessageContent::String(_))
    }

    pub fn is_user_prompt(&self) -> bool {
        self.role == Role::User
    }

    pub fn is_assistant_prompt(&self) -> bool {
        self.role == Role::Assistant
    }
}

/// README.md explains what pdl is
pub fn messages_from_pdl(
    content: String,
    context: tera::Context,
) -> Result<Vec<Message>, Error> {
    let rendered = tera::Tera::one_off(
        &content,
        &context,
        true,
    )?;
    let mut result = vec![];
    let mut buffer = vec![];
    let message_start = Regex::new(r"^<\|([a-zA-Z]+)\|>$").unwrap();
    let pdl_token = Regex::new(r"(.*)<\|([a-zA-Z0-9_()./,=+: ]+)\|>(.*)").unwrap();
    let mut curr_role = Role::System;

    for line in rendered.lines() {
        if let Some(c) = message_start.captures(line) {
            let content = buffer.join("\n").trim().to_string();

            if !content.is_empty() {
                result.push(Message {
                    role: curr_role,
                    content: parse_content(&content, &pdl_token)?,
                });
            }

            buffer = vec![];
            curr_role = Role::from_str(c.get(1).unwrap().as_str())?;
        }

        else {
            buffer.push(line);
        }
    }

    let content = buffer.join("\n").trim().to_string();

    if !content.is_empty() {
        result.push(Message {
            role: curr_role,
            content: parse_content(&content, &pdl_token)?,
        });
    }

    Ok(result)
}

/// It reads a pdl file and constructs messages
pub fn messages_from_file(
    file: &str,
    context: tera::Context,
) -> Result<Vec<Message>, Error> {
    messages_from_pdl(read_string(file)?, context)
}

fn parse_content(content: &str, pdl_token_re: &Regex) -> Result<Vec<MessageContent>, Error> {
    if let Some(c) = pdl_token_re.captures(content) {
        let prefix = c.get(1).unwrap().as_str();
        let token = c.get(2).unwrap().as_str();
        let postfix = c.get(3).unwrap().as_str();

        let media_re = Regex::new(r"\s*media\s*\((.*)\)").unwrap();
        let raw_media_re = Regex::new(r"\s*raw\_media\s*\(([a-z]+):([0-9A-Za-z+/=]+)\)").unwrap();

        if let Some(cap) = raw_media_re.captures(token) {
            let image_type = ImageType::from_extension(&cap[1])?;
            let bytes = decode_base64(&cap[2])?;

            Ok(vec![
                parse_content(prefix, pdl_token_re)?,
                vec![MessageContent::Image {
                    image_type,
                    bytes,
                }],
                parse_content(postfix, pdl_token_re)?,
            ].concat())
        }

        else if let Some(cap) = media_re.captures(token) {
            let path = cap[1].to_string();

            Ok(vec![
                parse_content(prefix, pdl_token_re)?,
                MediaMessageBuilder { paths: vec![path], prompt: None }.build()?,
                parse_content(postfix, pdl_token_re)?,
            ].concat())
        }

        else {
            Ok(vec![MessageContent::String(content.to_string())])
        }
    }

    else {
        if content.is_empty() {
            Ok(vec![])
        }

        else {
            Ok(vec![MessageContent::String(content.to_string())])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MessageContent, messages_from_file};
    use crate::{Message, Role};
    use ragit_fs::{WriteMode, write_string};

    #[test]
    fn messages_from_file_test() {
        write_string(
            "/tmp/test_messages.tera",
"
<|system|>

You're a code helper.

<|user|>

Write me a sudoku-solver.


",
            WriteMode::CreateOrTruncate,
        ).unwrap();

        let messages = messages_from_file("/tmp/test_messages.tera", tera::Context::new()).unwrap();

        assert_eq!(
            messages,
            vec![
                Message {
                    role: Role::System,
                    content: vec![
                        MessageContent::String(String::from("You're a code helper.")),
                    ],
                },
                Message {
                    role: Role::User,
                    content: vec![
                        MessageContent::String(String::from("Write me a sudoku-solver.")),
                    ],
                },
            ],
        );
    }
}
