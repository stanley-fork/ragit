use ragit_api as api;
use ragit_fs::join;
use serde::{Deserialize, Serialize};

pub const API_CONFIG_FILE_NAME: &str = "api.json";

// one that the user initializes
// it's later converted to `ApiConfig` by `Index`
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiConfigRaw {
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
            model: String::from("llama3.1-70b-groq"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ApiConfig {
    // if it's none, it searches the env var
    pub api_key: Option<String>,
    pub model: api::ChatModel,
    pub timeout: Option<u64>,  // milliseconds
    pub sleep_between_retries: u64,  // milliseconds
    pub max_retry: usize,

    pub dump_log_at: Option<String>,

    // dir of pdl files
    pub dump_api_usage_at: Option<String>,

    // in milliseconds
    // if you see 429 too often, use this option
    pub sleep_after_llm_call: Option<u64>,
}

impl ApiConfig {
    pub fn create_pdl_path(&self, job: &str) -> Option<String> {
        let now = h_time::Date::now();

        self.dump_log_at.as_ref().map(
            |path| join(
                path,
                &format!(
                    "{job}-{}_{:02}_{:02}-{:02}:{:02}:{:02}.pdl",
                    now.year,
                    now.month,
                    now.m_day,
                    now.hour,
                    now.minute,
                    now.second,
                ),
            ).unwrap()
        )
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
