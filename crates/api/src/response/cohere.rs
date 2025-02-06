use super::{IntoChatResponse, Response};
use crate::error::Error;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct CohereResponse {
    id: String,
    finish_reason: String,
    message: CohereMessage,
    usage: CohereUsage,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CohereMessage {
    role: String,
    content: Vec<CohereContent>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CohereUsage {
    billed_units: CohereTokens,
    tokens: CohereTokens,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CohereContent {
    r#type: String,
    text: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CohereTokens {
    input_tokens: usize,
    output_tokens: usize,
}

impl IntoChatResponse for CohereResponse {
    fn into_chat_response(&self) -> Result<Response, Error> {
        Ok(Response {
            messages: vec![self.message.content[0].text.to_string()],
            reasonings: self.message.content.iter().map(|_| None).collect(),
            output_tokens: self.usage.tokens.output_tokens,
            prompt_tokens: self.usage.tokens.input_tokens,
            total_tokens: self.usage.tokens.output_tokens + self.usage.tokens.input_tokens,
        })
    }
}
