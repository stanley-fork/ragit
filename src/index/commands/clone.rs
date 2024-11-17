use super::Index;
use crate::chunk;
use crate::error::Error;
use crate::index::{
    CHUNK_DIR_NAME,
    IMAGE_DIR_NAME,
    INDEX_DIR_NAME,
    LoadMode,
    METADATA_FILE_NAME,
};
use json::JsonValue;
use ragit_api::{JsonType, get_type};
use ragit_fs::{
    WriteMode,
    create_dir,
    join3,
    join4,
    remove_dir_all,
    set_extension,
    write_bytes,
};
use reqwest::Url;

struct CloneState {
    url: String,
    image_total: usize,
    image_count: usize,
    chunk_total: usize,
    chunk_count: usize,
}

impl Index {
    pub async fn clone(url: String, repo_name: Option<String>) -> Result<(), Error> {
        let repo_name = repo_name.unwrap_or_else(|| infer_repo_name_from_url(&url));
        create_dir(&repo_name)?;

        match Index::clone_worker(url, repo_name.clone()).await {
            Ok(()) => Ok(()),
            Err(e) => {
                remove_dir_all(&repo_name)?;
                Err(e)
            },
        }
    }

    async fn clone_worker(mut url: String, repo_name: String) -> Result<(), Error> {
        todo!()
    }

    // TODO: erase lines instead of the entire screen
    fn render_clone_dashboard(state: &CloneState) {
        clearscreen::clear().expect("failed to clear screen");
        println!("cloning {}...", state.url);
        println!("chunks: {}/{}", state.chunk_count, state.chunk_total);
        println!("images: {}/{}", state.image_count, state.image_total);
    }
}

// TODO: it's too naive
fn infer_repo_name_from_url(url: &str) -> String {
    // This function doesn't need any error-handling
    // because if any of these fail, `Index::clone_worker()`
    // would also fail and there's an error handler for
    // `Index::clone_worker()`.
    match Url::parse(url) {
        Ok(url) => match url.path_segments() {
            Some(paths) => match paths.last() {
                Some(name) => name.to_string(),
                _ => String::from("_"),
            },
            _ => String::from("_"),
        },
        _ => String::from("_"),
    }
}

async fn request_json_file(url: &str) -> Result<JsonValue, Error> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if response.status().as_u16() != 200 {
        return Err(Error::CloneRequestError {
            code: Some(response.status().as_u16()),
            url: url.to_string(),
        });
    }

    Ok(json::parse(&response.text().await?)?)
}

async fn request_binary_file(url: &str) -> Result<Vec<u8>, Error> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if response.status().as_u16() != 200 {
        return Err(Error::CloneRequestError {
            code: Some(response.status().as_u16()),
            url: url.to_string(),
        });
    }

    Ok(response.bytes().await?.to_vec())
}

// TODO: create a fork of <https://crates.io/crates/json>, then impl
//       1. this feature (with much fancier api)
//       2, sortable objects (for dump results)
fn parse_vec_string(j: &JsonValue) -> Result<Vec<String>, Error> {
    match j {
        JsonValue::Array(values) => {
            let mut result = Vec::with_capacity(values.len());

            for value in values.iter() {
                match value.as_str() {
                    Some(s) => { result.push(s.to_string()); },
                    None => {
                        return Err(Error::JsonTypeError {
                            expected: JsonType::String,
                            got: get_type(value),
                        });
                    },
                }
            }

            Ok(result)
        },
        _ => Err(Error::JsonTypeError {
            expected: JsonType::Array,
            got: get_type(j),
        }),
    }
}
