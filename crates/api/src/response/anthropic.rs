use super::{IntoChatResponse, Response};
use crate::error::Error;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<AnthropicContent>,
    role: String,
    stop_reason: String,
    r#type: String,
    usage: AnthropicUsage,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
    r#type: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}

impl IntoChatResponse for AnthropicResponse {
    fn into_chat_response(&self) -> Result<Response, Error> {
        Ok(Response {
            messages: self.content.iter().map(
                |content| content.text.to_string()
            ).collect(),
            reasonings: self.content.iter().map(|_| None).collect(),
            output_tokens: self.usage.output_tokens,
            prompt_tokens: self.usage.input_tokens,
            total_tokens: self.usage.output_tokens + self.usage.input_tokens,
        })
    }
}
