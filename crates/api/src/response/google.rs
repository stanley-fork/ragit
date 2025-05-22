use super::{IntoChatResponse, Response};
use crate::error::Error;
use serde::Deserialize;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct GoogleResponse {
    candidates: Vec<GoogleCandidate>,
    usageMetadata: GoogleUsageMetadata,
    modelVersion: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct GoogleCandidate {
    content: GoogleContent,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct GoogleContent {
    parts: Vec<GooglePart>,
    role: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct GooglePart {
    thought: Option<bool>,
    text: Option<String>,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct GoogleUsageMetadata {
    promptTokenCount: usize,
    candidatesTokenCount: usize,
    totalTokenCount: usize,
}

impl IntoChatResponse for GoogleResponse {
    fn into_chat_response(&self) -> Result<Response, Error> {
        Ok(Response {
            messages: self.candidates.iter().map(
                |candidate| candidate.content.parts.iter().filter(
                    |p| !p.thought.unwrap_or(false)
                ).map(
                    |p| p.text.clone().unwrap_or(String::new())
                ).collect::<Vec<_>>().concat()
            ).filter(
                |candidate| !candidate.is_empty()
            ).collect(),
            reasonings: self.candidates.iter().map(
                |candidate| candidate.content.parts.iter().filter(
                    |p| p.thought.unwrap_or(false)
                ).map(
                    |p| p.text.clone().unwrap_or(String::new())
                ).collect::<Vec<_>>().concat()
            ).map(
                |candidate| if candidate.is_empty() {
                    None
                } else {
                    Some(candidate)
                }
            ).collect(),
            output_tokens: self.usageMetadata.candidatesTokenCount,
            prompt_tokens: self.usageMetadata.promptTokenCount,
            total_tokens: self.usageMetadata.totalTokenCount
        })
    }
}
