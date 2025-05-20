use crate::error::Error;
use ragit_api::ModelRaw;
use ragit_fs::read_string;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    // If set, it dumps log to this file.
    pub log_file: Option<String>,

    // If set, it dumps log to stdout.
    // `log` and `dump_log_to_stdout` are independent to each other.
    pub dump_log_to_stdout: bool,

    // A directory where push sessions are stored.
    pub push_session_dir: String,
    pub repo_data_dir: String,
    pub blob_dir: String,

    // A path to `models.json` file. If not found, it'll try to create a file
    // in the given path.
    pub default_models: String,

    // Name of a default ai model. The model must be defined in `default_models` file.
    pub default_ai_model: String,

    pub only_admin_can_create_user: bool,
    pub port_number: u16,
}

impl Config {
    pub fn load_from_file(file: &str) -> Result<Config, Error> {
        let j = read_string(file)?;
        Ok(serde_json::from_str(&j)?)
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            log_file: Some(String::from("./ragit-server-logs")),
            dump_log_to_stdout: false,
            default_models: String::from("./models.json"),
            default_ai_model: String::from("llama3.3"),
            push_session_dir: String::from("./session"),
            repo_data_dir: String::from("./data"),
            blob_dir: String::from("./blobs"),
            only_admin_can_create_user: true,
            port_number: 41127,
        }
    }
}

// It's not saved as a file. It's constructed on the fly from the
// config file and model file.
#[derive(Debug)]
pub struct AiModelConfig {
    pub default_models: Vec<ModelRaw>,
    pub default_model: String,
}
