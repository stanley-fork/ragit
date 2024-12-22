use chrono::offset::Local;
use crate::error::Error;
use ragit_api::{self as api, record::{Record, Tracker}};
use ragit_fs::join;
use serde::{Deserialize, Serialize};

pub const API_CONFIG_FILE_NAME: &str = "api.json";

// one that the user initializes
// it's later converted to `ApiConfig` by `Index`
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ApiConfigRaw {
    // I recommend you use env var, instead of this.
    pub api_key: Option<String>,

    // run `rag ls --models` to see the list
    pub model: String,
    pub timeout: Option<u64>,
    pub sleep_between_retries: u64,
    pub max_retry: usize,
    pub sleep_after_llm_call: Option<u64>,

    // it records every LLM conversation, including failed ones
    // it's useful if you wanna know what's going on!
    // but be careful, it would take a lot of space
    pub dump_log: bool,

    // it records how many tokens are used
    pub dump_api_usage: bool,
}

impl Default for ApiConfigRaw {
    fn default() -> Self {
        ApiConfigRaw {
            api_key: None,
            dump_log: false,
            dump_api_usage: true,
            max_retry: 3,
            sleep_between_retries: 20_000,
            timeout: Some(90_000),
            sleep_after_llm_call: None,
            model: String::from("llama3.3-70b-groq"),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ApiConfig {
    // if it's none, it searches the env var
    pub api_key: Option<String>,
    pub model: api::ChatModel,
    pub timeout: Option<u64>,  // milliseconds
    pub sleep_between_retries: u64,  // milliseconds
    pub max_retry: usize,

    // dir of pdl files
    pub dump_log_at: Option<String>,
    pub dump_api_usage_at: Option<String>,

    // in milliseconds
    // if you see 429 too often, use this option
    pub sleep_after_llm_call: Option<u64>,
}

impl ApiConfig {
    pub fn create_pdl_path(&self, job: &str) -> Option<String> {
        let now = Local::now();

        self.dump_log_at.as_ref().map(
            |path| join(
                path,
                &format!(
                    "{job}-{}.pdl",
                    now.to_rfc3339(),
                ),
            ).unwrap()
        )
    }

    pub fn get_api_usage(&self, id: &str) -> Result<Vec<Record>, Error> {
        match &self.dump_api_usage_at {
            Some(path) => {
                let tracker = Tracker::load_from_file(path)?;

                match tracker.0.get(id) {
                    Some(record) => Ok(record.clone()),

                    // It's not an error, it's just that this id was never used
                    None => Ok(vec![]),
                }
            },
            None => Ok(vec![]),  // TODO: is this an error attempting to do this? I'm not sure
        }
    }
}

impl Default for ApiConfig {
    // this is just a tmp placeholder
    // every ApiConfig must be initialized by `Index::init_api_config(&ApiConfigRaw)`
    fn default() -> Self {
        ApiConfig {
            api_key: None,
            model: api::ChatModel::Sonnet,
            timeout: None,
            sleep_between_retries: 0,
            max_retry: 0,
            dump_log_at: None,
            dump_api_usage_at: None,
            sleep_after_llm_call: None,
        }
    }
}
