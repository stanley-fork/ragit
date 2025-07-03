pub use super::{BuildConfig, Index};
use crate::error::Error;
use serde_json::Value;
use std::io::Write;

// If a command dumps anything to stdout, its method must have `quiet: bool` argument.
mod add;
mod archive;
mod audit;
mod build;
mod check;
mod clone;
mod config;
mod gc;
mod ls;
mod merge;
mod meta;
mod migrate;
mod model;
mod pull;
mod push;
mod recover;
mod remove;
mod summary;
mod uid;

pub use add::{AddMode, AddResult};
pub use audit::Audit;
pub use build::BuildResult;
pub use merge::{MergeMode, MergeResult};
pub use migrate::{VersionInfo, get_compatibility_warning};
pub use pull::PullResult;
pub use push::PushResult;
pub use recover::RecoverResult;
pub use remove::RemoveResult;
pub use summary::{Summary, SummaryMode};

pub fn erase_lines(n: usize) {
    if n != 0 {
        print!("\x1B[{n}A");
        print!("\x1B[J");
        std::io::stdout().flush().unwrap();
    }
}

pub async fn request_binary_file(url: &str) -> Result<Vec<u8>, Error> {
    let client = reqwest::Client::new();
    let mut request = client.get(url);

    if let Some(api_key) = get_ragit_api_key() {
        request = request.header(
            "x-api-key",
            &api_key,
        );
    }

    let response = request.send().await?;

    if response.status().as_u16() != 200 {
        return Err(Error::RequestFailure {
            context: Some(String::from("clone")),
            code: Some(response.status().as_u16()),
            url: url.to_string(),
        });
    }

    Ok(response.bytes().await?.to_vec())
}

pub async fn request_json_file(url: &str) -> Result<Value, Error> {
    let client = reqwest::Client::new();
    let mut request = client.get(url);

    if let Some(api_key) = get_ragit_api_key() {
        request = request.header(
            "x-api-key",
            &api_key,
        );
    }

    let response = request.send().await?;

    if response.status().as_u16() != 200 {
        return Err(Error::RequestFailure {
            context: Some(String::from("clone")),
            code: Some(response.status().as_u16()),
            url: url.to_string(),
        });
    }

    Ok(serde_json::from_str(&response.text().await?)?)
}

pub fn get_ragit_api_key() -> Option<String> {
    std::env::var("RAGIT_API_KEY").ok()
}
