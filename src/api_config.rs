use chrono::offset::Local;
use crate::constant::{INDEX_DIR_NAME, LOG_DIR_NAME};
use crate::error::Error;
use ragit_api::record::{Record, RecordAt, Tracker};
use ragit_fs::{
    WriteMode,
    exists,
    join,
    join3,
    write_string,
};
use serde::{Deserialize, Serialize};

// This struct is used for loading partial configurations from ~/.config/ragit/api.json
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct PartialApiConfig {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub timeout: Option<u64>,
    pub sleep_between_retries: Option<u64>,
    pub max_retry: Option<usize>,
    pub sleep_after_llm_call: Option<u64>,
    pub dump_log: Option<bool>,
    pub dump_api_usage: Option<bool>,
}

impl PartialApiConfig {
    // Apply partial config to a full config
    pub fn apply_to(&self, config: &mut ApiConfig) {
        if let Some(api_key) = &self.api_key {
            config.api_key = Some(api_key.clone());
        }
        if let Some(model) = &self.model {
            config.model = model.clone();
        }
        if let Some(timeout) = self.timeout {
            config.timeout = Some(timeout);
        }
        if let Some(sleep_between_retries) = self.sleep_between_retries {
            config.sleep_between_retries = sleep_between_retries.clone();
        }
        if let Some(max_retry) = self.max_retry {
            config.max_retry = max_retry;
        }
        if let Some(sleep_after_llm_call) = self.sleep_after_llm_call {
            config.sleep_after_llm_call = Some(sleep_after_llm_call);
        }
        if let Some(dump_log) = self.dump_log {
            config.dump_log = dump_log.clone();
        }
        if let Some(dump_api_usage) = self.dump_api_usage {
            config.dump_api_usage = dump_api_usage.clone();
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ApiConfig {
    // This value is NOT used anymore. It's here for backward-compatibility
    // You have to use env var or `models.json`.
    pub api_key: Option<String>,

    // run `rag ls-models` to see the list
    pub model: String,
    pub timeout: Option<u64>,
    pub sleep_between_retries: u64,
    pub max_retry: usize,

    // in milliseconds
    // if you see 429 too often, use this option
    pub sleep_after_llm_call: Option<u64>,

    // it records every LLM conversation, including failed ones
    // it's useful if you wanna know what's going on!
    // but be careful, it would take a lot of space
    pub dump_log: bool,

    // it records how many tokens are used
    pub dump_api_usage: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            api_key: None,
            dump_log: false,
            dump_api_usage: true,
            max_retry: 5,
            sleep_between_retries: 15_000,
            timeout: Some(120_000),
            sleep_after_llm_call: None,
            model: String::from("llama3.3-70b-groq"),
        }
    }
}

impl ApiConfig {
    pub fn create_pdl_path(&self, root_dir: &str, job: &str) -> Option<String> {
        let now = Local::now();

        self.dump_log_at(root_dir).as_ref().map(
            |path| join(
                path,
                &format!(
                    "{job}-{}.pdl",
                    now.to_rfc3339(),
                ),
            ).unwrap()
        )
    }

    pub fn dump_log_at(&self, root_dir: &str) -> Option<String> {
        if self.dump_log {
            join3(root_dir, INDEX_DIR_NAME, LOG_DIR_NAME).ok()
        }

        else {
            None
        }
    }

    pub fn dump_api_usage_at(&self, root_dir: &str, id: &str) -> Option<RecordAt> {
        if self.dump_api_usage {
            match join3(root_dir, INDEX_DIR_NAME, "usages.json") {
                Ok(path) => {
                    if !exists(&path) {
                        let _ = write_string(
                            &path,
                            "{}",
                            WriteMode::AlwaysCreate,
                        );
                    }

                    Some(RecordAt { path, id: id.to_string() })
                },
                Err(_) => None,
            }
        }

        else {
            None
        }
    }

    pub fn get_api_usage(&self, root_dir: &str, id: &str) -> Result<Vec<Record>, Error> {
        match &self.dump_api_usage_at(root_dir, id) {
            Some(RecordAt { path, id }) => {
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
