use super::ImageModel;
use crate::chat::{
    AnthropicResponse,
    CohereResponse,
    GroqResponse,
    IntoChatResponse,
    OllamaResponse,
    OpenAiResponse,
};
use crate::error::Error;

// TODO: openai-compatible apis?
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ApiProvider {
    OpenAi,
    Groq,
    Cohere,
    Replicate,
    Anthropic,
    Ollama,

    /// for test
    /// 1. doesn't require api key
    /// 2. needs no network
    Dummy,
}

impl ApiProvider {
    // why not Debug?
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiProvider::Anthropic => "Anthropic",
            ApiProvider::Cohere => "Cohere",
            ApiProvider::Groq => "Groq",
            ApiProvider::Ollama => "ollama",
            ApiProvider::OpenAi => "OpenAi",
            ApiProvider::Replicate => "Replicate",
            ApiProvider::Dummy => "dummy",
        }
    }

    pub fn get_chat_api_url(&self) -> Option<&'static str> {
        match self {
            ApiProvider::Anthropic => Some("https://api.anthropic.com/v1/messages"),
            ApiProvider::Cohere => Some("https://api.cohere.com/v2/chat"),
            ApiProvider::Groq => Some("https://api.groq.com/openai/v1/chat/completions"),
            ApiProvider::Ollama => Some("http://127.0.0.1:11434/v1/chat/completions"),
            ApiProvider::OpenAi => Some("https://api.openai.com/v1/chat/completions"),
            ApiProvider::Replicate => None,
            ApiProvider::Dummy => None,
        }
    }

    pub fn get_image_create_api_url(&self, model_kind: ImageModel) -> Option<String> {
        match self {
            ApiProvider::Anthropic => None,
            ApiProvider::Cohere => None,
            ApiProvider::Groq => None,
            ApiProvider::Ollama => None,
            ApiProvider::OpenAi => Some(String::from("https://api.openai.com/v1/images/generations")),
            ApiProvider::Replicate => if model_kind.uses_version_hash() {
                Some(String::from("https://api.replicate.com/v1/predictions"))
            } else {
                // TODO: make "black-forest-labs" configurable
                Some(format!("https://api.replicate.com/v1/models/black-forest-labs/{}/predictions", model_kind.to_api_friendly_name()))
            },
            ApiProvider::Dummy => None,
        }
    }

    pub fn get_image_get_api_url(&self, id: &str, url: &Option<String>) -> Option<String> {
        match self {
            ApiProvider::Replicate => Some(format!("https://api.replicate.com/v1/predictions/{}", id)),
            ApiProvider::OpenAi => url.as_ref().map(|u| u.to_string()),
            _ => None,
        }
    }

    pub fn api_key_env_var(&self) -> Option<&str> {
        match self {
            ApiProvider::Anthropic => Some("ANTHROPIC_API_KEY"),
            ApiProvider::Cohere => Some("COHERE_API_KEY"),
            ApiProvider::Groq => Some("GROQ_API_KEY"),
            ApiProvider::OpenAi => Some("OPENAI_API_KEY"),
            ApiProvider::Replicate => Some("REPLICATE_API_KEY"),
            ApiProvider::Ollama
            | ApiProvider::Dummy => None,
        }
    }

    // it panics if env var is not found
    pub fn get_api_key_from_env(&self) -> String {
        match self.api_key_env_var() {
            Some(var) => std::env::var(var).unwrap_or_else(
                |_| panic!("env var not found: {var}")
            ),
            None => String::new(),
        }
    }

    pub fn parse_chat_response(&self, s: &str) -> Result<Box<dyn IntoChatResponse>, Error> {
        match self {
            ApiProvider::Anthropic => Ok(Box::new(serde_json::from_str::<AnthropicResponse>(s)?)),
            ApiProvider::Cohere => Ok(Box::new(serde_json::from_str::<CohereResponse>(s)?)),
            ApiProvider::Groq => Ok(Box::new(serde_json::from_str::<GroqResponse>(s)?)),
            ApiProvider::OpenAi => Ok(Box::new(serde_json::from_str::<OpenAiResponse>(s)?)),
            ApiProvider::Ollama => Ok(Box::new(serde_json::from_str::<OllamaResponse>(s)?)),
            ApiProvider::Replicate => unreachable!(),
            ApiProvider::Dummy => unreachable!(),
        }
    }
}
