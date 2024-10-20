mod message;
mod model_kind;
mod request;
mod response;

pub use message::{
    ImageType,
    MediaMessageBuilder,
    Message,
    MessageContent,
    message_contents_to_json_array,
    message_contents_to_string,
    messages_from_file,
    messages_from_pdl,
};
pub use model_kind::ModelKind;
pub use request::Request;
pub use response::{
    AnthropicResponse,
    CohereResponse,
    GroqResponse,
    IntoChatResponse,
    OllamaResponse,
    OpenAiResponse,
    Response,
};

use crate::{ApiProvider, Error};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    pub fn to_api_string(&self, api_provider: ApiProvider) -> &'static str {
        match (self, api_provider) {
            (
                Role::User,
                ApiProvider::Anthropic
                | ApiProvider::Cohere
                | ApiProvider::Groq
                | ApiProvider::Ollama
                | ApiProvider::OpenAi,
            ) => "user",
            (
                Role::Assistant,
                ApiProvider::Anthropic
                | ApiProvider::Cohere
                | ApiProvider::Groq
                | ApiProvider::Ollama
                | ApiProvider::OpenAi,
            ) => "assistant",
            (
                Role::System,
                ApiProvider::Anthropic
                | ApiProvider::Cohere
                | ApiProvider::Groq
                | ApiProvider::Ollama
                | ApiProvider::OpenAi,
            ) => "system",
            (
                _,
                ApiProvider::Replicate  // for now, there's no chat model for replicate
                | ApiProvider::Dummy,
            ) => unreachable!(),
        }
    }
}

impl FromStr for Role {
    type Err = Error;

    fn from_str(s: &str) -> Result<Role, Error> {
        match s.to_ascii_lowercase() {
            s if s == "user" => Ok(Role::User),
            s if s == "assistant" || s == "chatbot" => Ok(Role::Assistant),
            s if s == "system" => Ok(Role::System),
            _ => Err(Error::InvalidRole(s.to_string())),
        }
    }
}
