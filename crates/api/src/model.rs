use crate::api_provider::ApiProvider;
use crate::error::Error;
use lazy_static::lazy_static;
use ragit_fs::join4;
use ragit_pdl::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write, stdin, stdout};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Model {
    pub name: String,
    pub api_name: String,
    pub can_read_images: bool,
    pub api_provider: ApiProvider,
    pub dollars_per_1b_input_tokens: u64,
    pub dollars_per_1b_output_tokens: u64,
    pub api_timeout: u64,
    pub explanation: Option<String>,
    pub api_key: Option<String>,
    pub api_env_var: Option<String>,
}

impl Model {
    /// This is a test model. It always returns a string `"dummy"`.
    pub fn dummy() -> Self {
        Model {
            name: String::from("dummy"),
            api_name: String::from("test-model-dummy-v0"),
            can_read_images: false,
            api_provider: ApiProvider::Test(TestModel::Dummy),
            dollars_per_1b_input_tokens: 0,
            dollars_per_1b_output_tokens: 0,
            api_timeout: 180,
            explanation: None,
            api_key: None,
            api_env_var: None,
        }
    }

    /// This is a test model. It takes a response from you.
    pub fn stdin() -> Self {
        Model {
            name: String::from("stdin"),
            api_name: String::from("test-model-stdin-v0"),
            can_read_images: false,
            api_provider: ApiProvider::Test(TestModel::Stdin),
            dollars_per_1b_input_tokens: 0,
            dollars_per_1b_output_tokens: 0,
            api_timeout: 180,
            explanation: None,
            api_key: None,
            api_env_var: None,
        }
    }

    /// This is a test model. It always throws an error.
    pub fn error() -> Self {
        Model {
            name: String::from("error"),
            api_name: String::from("test-model-error-v0"),
            can_read_images: false,
            api_provider: ApiProvider::Test(TestModel::Error),
            dollars_per_1b_input_tokens: 0,
            dollars_per_1b_output_tokens: 0,
            api_timeout: 180,
            explanation: None,
            api_key: None,
            api_env_var: None,
        }
    }

    pub fn get_api_url(&self) -> Result<String, Error> {
        let url = match &self.api_provider {
            ApiProvider::Anthropic => String::from("https://api.anthropic.com/v1/messages"),
            ApiProvider::Cohere => String::from("https://api.cohere.com/v2/chat"),
            ApiProvider::OpenAi { url } => url.to_string(),
            ApiProvider::Google => format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                self.api_name,
                self.get_api_key()?,
            ),
            ApiProvider::Test(_) => String::new(),
        };

        Ok(url)
    }

    pub fn get_api_key(&self) -> Result<String, Error> {
        // First, check if the API key is directly set in the model
        if let Some(key) = &self.api_key {
            return Ok(key.to_string());
        }

        // Next, check if an environment variable is specified and try to get the API key from it
        if let Some(var) = &self.api_env_var {
            if let Ok(key) = std::env::var(var) {
                return Ok(key.to_string());
            }

            // Don't return an error yet, try the other methods first
        }

        // If we get here, try to find the API key in external model files
        if let Some(key) = self.find_api_key_in_external_files()? {
            return Ok(key);
        }

        // If we have an api_env_var but couldn't find the key anywhere, return an error
        if let Some(var) = &self.api_env_var {
            return Err(Error::ApiKeyNotFound { env_var: Some(var.to_string()) });
        }

        // If both `api_key` and `api_env_var` are not set,
        // it assumes that the model does not require an
        // api key.
        Ok(String::new())
    }

    fn find_api_key_in_external_files(&self) -> Result<Option<String>, Error> {
        // Try to find the API key in the file indicated by RAGIT_MODEL_FILE
        if let Ok(file_path) = std::env::var("RAGIT_MODEL_FILE") {
            if let Some(key) = self.find_api_key_in_file(&file_path)? {
                return Ok(Some(key));
            }
        }

        // Try to find the API key in ~/.config/ragit/models.json
        if let Ok(home_dir) = std::env::var("HOME") {
            let config_path = join4(
                &home_dir,
                ".config",
                "ragit",
                "models.json",
            )?;

            if let Some(key) = self.find_api_key_in_file(&config_path)? {
                return Ok(Some(key));
            }
        }

        Ok(None)
    }

    fn find_api_key_in_file(&self, file_path: &str) -> Result<Option<String>, Error> {
        use std::fs::File;
        use std::io::Read;

        // Check if the file exists
        let file = match File::open(file_path) {
            Ok(file) => file,
            Err(_) => return Ok(None), // File doesn't exist or can't be opened
        };

        // Read the file content
        let mut content = String::new();
        if let Err(_) = file.take(10_000_000).read_to_string(&mut content) {
            return Ok(None); // Can't read the file
        }

        // Parse the JSON
        let models: Vec<ModelRaw> = match serde_json::from_str(&content) {
            Ok(models) => models,
            Err(_) => return Ok(None), // Can't parse the JSON
        };

        // Find the model with the same name
        for model in models {
            if model.name == self.name {
                // If the model has an API key, return it
                if let Some(key) = model.api_key {
                    return Ok(Some(key));
                }

                // If the model has an environment variable, try to get the API key from it
                if let Some(var) = model.api_env_var {
                    if let Ok(key) = std::env::var(&var) {
                        return Ok(Some(key));
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn is_test_model(&self) -> bool {
        matches!(self.api_provider, ApiProvider::Test(_))
    }

    pub fn default_models() -> Vec<Model> {
        ModelRaw::default_models().iter().map(
            |model| model.try_into().unwrap()
        ).collect()
    }
}

/// There are 2 types for models: `Model` and `ModelRaw`. I know it's confusing, I'm sorry.
/// `Model` is the type ragit internally uses and `ModelRaw` is only for json serialization.
/// Long time ago, there was only `Model` type. But then I implemented `models.json` interface.
/// I wanted people to directly edit the json file and found that `Model` isn't intuitive to
/// edit directly. So I added this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModelRaw {
    /// Model name shown to user.
    /// `rag config --set model` also
    /// uses this name.
    pub name: String,

    /// Model name used for api requests.
    pub api_name: String,

    pub can_read_images: bool,

    /// `openai | cohere | anthropic | google`
    ///
    /// If you're using an openai-compatible
    /// api, set this to `openai`.
    pub api_provider: String,

    /// It's necessary if you're using an
    /// openai-compatible api. If it's not
    /// set, ragit uses the default url of
    /// each api provider.
    pub api_url: Option<String>,

    /// Dollars per 1 million input tokens.
    pub input_price: f64,

    /// Dollars per 1 million output tokens.
    pub output_price: f64,

    // FIXME: I set the default value to 180 seconds long ago.
    //        At that time, it's very common for LLMs to take
    //        1 ~ 2 minutes to respond. But now, nobody would
    //        wait 180 seconds. Do I have to reduce it?
    /// The number is in seconds.
    /// If not set, it's default to 180 seconds.
    #[serde(default)]
    pub api_timeout: Option<u64>,

    pub explanation: Option<String>,

    /// If you don't want to use an env var, you
    /// can hard-code your api key in this field.
    #[serde(default)]
    pub api_key: Option<String>,

    /// If you've hard-coded your api key,
    /// you don't have to set this. If neither
    /// `api_key`, nor `api_env_var` is set,
    /// it assumes that the model doesn't require
    /// an api key.
    pub api_env_var: Option<String>,
}

lazy_static! {
    static ref DEFAULT_MODELS: HashMap<String, ModelRaw> = {
        let models_dot_json = include_str!("../models.json");
        let models = serde_json::from_str::<Vec<ModelRaw>>(&models_dot_json).unwrap();
        models.into_iter().map(
            |model| (model.name.clone(), model)
        ).collect()
    };
}

impl ModelRaw {
    pub fn llama_70b() -> Self {
        DEFAULT_MODELS.get("llama3.3-70b-groq").unwrap().clone()
    }

    pub fn llama_8b() -> Self {
        DEFAULT_MODELS.get("llama3.1-8b-groq").unwrap().clone()
    }

    pub fn gpt_4o() -> Self {
        DEFAULT_MODELS.get("gpt-4o").unwrap().clone()
    }

    pub fn gpt_4o_mini() -> Self {
        DEFAULT_MODELS.get("gpt-4o-mini").unwrap().clone()
    }

    pub fn gemini_2_flash() -> Self {
        DEFAULT_MODELS.get("gemini-2.0-flash").unwrap().clone()
    }

    pub fn sonnet() -> Self {
        DEFAULT_MODELS.get("claude-3.7-sonnet").unwrap().clone()
    }

    pub fn phi_4_14b() -> Self {
        DEFAULT_MODELS.get("phi-4-14b-ollama").unwrap().clone()
    }

    pub fn command_r() -> Self {
        DEFAULT_MODELS.get("command-r").unwrap().clone()
    }

    pub fn command_r_plus() -> Self {
        DEFAULT_MODELS.get("command-r-plus").unwrap().clone()
    }

    pub fn default_models() -> Vec<ModelRaw> {
        DEFAULT_MODELS.values().map(|model| model.clone()).collect()
    }
}

pub fn get_model_by_name(models: &[Model], name: &str) -> Result<Model, Error> {
    let mut partial_matches = vec![];

    for model in models.iter() {
        if model.name == name {
            return Ok(model.clone());
        }

        if partial_match(&model.name, name) {
            partial_matches.push(model);
        }
    }

    if partial_matches.len() == 1 {
        Ok(partial_matches[0].clone())
    }

    else if name == "dummy" {
        Ok(Model::dummy())
    }

    else if name == "stdin" {
        Ok(Model::stdin())
    }

    else if name == "error" {
        Ok(Model::error())
    }

    else{
        Err(Error::InvalidModelName {
            name: name.to_string(),
            candidates: partial_matches.iter().map(
                |model| model.name.to_string()
            ).collect(),
        })
    }
}

impl TryFrom<&ModelRaw> for Model {
    type Error = Error;

    fn try_from(m: &ModelRaw) -> Result<Model, Error> {
        Ok(Model {
            name: m.name.clone(),
            api_name: m.api_name.clone(),
            can_read_images: m.can_read_images,
            api_provider: ApiProvider::parse(
                &m.api_provider,
                &m.api_url,
            )?,
            dollars_per_1b_input_tokens: (m.input_price * 1000.0).round() as u64,
            dollars_per_1b_output_tokens: (m.output_price * 1000.0).round() as u64,
            api_timeout: m.api_timeout.unwrap_or(180),
            explanation: m.explanation.clone(),
            api_key: m.api_key.clone(),
            api_env_var: m.api_env_var.clone(),
        })
    }
}

impl From<&Model> for ModelRaw {
    fn from(m: &Model) -> ModelRaw {
        ModelRaw {
            name: m.name.clone(),
            api_name: m.api_name.clone(),
            can_read_images: m.can_read_images,
            api_provider: m.api_provider.to_string(),

            // This field is for openai-compatible apis. The other api
            // providers do not need this field. The problem is that
            // `m.get_api_url()` may fail if api provider is google.
            // So it just ignores errors.
            api_url: m.get_api_url().ok(),

            input_price: m.dollars_per_1b_input_tokens as f64 / 1000.0,
            output_price: m.dollars_per_1b_output_tokens as f64 / 1000.0,
            api_timeout: Some(m.api_timeout),
            explanation: m.explanation.clone(),
            api_key: m.api_key.clone(),
            api_env_var: m.api_env_var.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TestModel {
    Dummy,  // it always returns `"dummy"`
    Stdin,
    Error,  // it always raises an error
}

impl TestModel {
    pub fn get_dummy_response(&self, messages: &[Message]) -> Result<String, Error> {
        match self {
            TestModel::Dummy => Ok(String::from("dummy")),
            TestModel::Stdin => {
                for message in messages.iter() {
                    println!(
                        "<|{:?}|>\n\n{}\n\n",
                        message.role,
                        message.content.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(""),
                    );
                }

                print!("<|Assistant|>\n\n>>> ");
                stdout().flush()?;

                let mut s = String::new();
                stdin().read_to_string(&mut s)?;
                Ok(s)
            },
            TestModel::Error => Err(Error::TestModel),
        }
    }
}

fn partial_match(haystack: &str, needle: &str) -> bool {
    let h_bytes = haystack.bytes().collect::<Vec<_>>();
    let n_bytes = needle.bytes().collect::<Vec<_>>();
    let mut h_cursor = 0;
    let mut n_cursor = 0;

    while h_cursor < h_bytes.len() && n_cursor < n_bytes.len() {
        if h_bytes[h_cursor] == n_bytes[n_cursor] {
            h_cursor += 1;
            n_cursor += 1;
        }

        else {
            h_cursor += 1;
        }
    }

    n_cursor == n_bytes.len()
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_MODELS, Model};

    #[test]
    fn validate_models_dot_json() {
        for model in DEFAULT_MODELS.values() {
            Model::try_from(model).unwrap();
        }
    }
}
