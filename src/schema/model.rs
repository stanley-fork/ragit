use super::Prettify;
use crate::error::Error;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
pub struct ModelSchema {
    pub name: String,
    pub api_provider: String,
    pub api_key_env_var: Option<String>,
    pub can_read_images: bool,
    pub dollars_per_1b_input_tokens: u64,
    pub dollars_per_1b_output_tokens: u64,
    pub explanation: String,
}

impl Prettify for ModelSchema {
    fn prettify(&self) -> Result<Value, Error> {
        Ok(serde_json::to_value(self)?)
    }
}
