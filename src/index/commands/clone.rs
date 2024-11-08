use super::Index;
use crate::chunk;
use crate::error::Error;
use crate::index::{CHUNK_DIR_NAME, IMAGE_DIR_NAME, INDEX_DIR_NAME, METADATA_FILE_NAME, UpdateTfidf};
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

    // TODO: dashboard
    // TODO: configs, prompts
    async fn clone_worker(mut url: String, repo_name: String) -> Result<(), Error> {
        if !url.ends_with("/") {
            url = format!("{url}/");
        }

        let url = Url::parse(&url)?;
        Index::new(repo_name.clone())?;

        let index_url = url.join("index/")?;
        let index_json = request_binary_file(index_url.as_str()).await?;

        write_bytes(
            &join3(
                &repo_name,
                &INDEX_DIR_NAME,
                "index.json",
            )?,
            &index_json,
            WriteMode::CreateOrTruncate,
        )?;
        let chunk_file_list_url = url.join("chunk-file-list")?;
        let chunk_file_list = request_json_file(chunk_file_list_url.as_str()).await?;
        let chunk_file_list = parse_vec_string(&chunk_file_list)?;

        for chunk_file in chunk_file_list.iter() {
            let chunk_file_url = url.join(&format!("chunk-file/{chunk_file}"))?;
            let chunk_file_data = request_binary_file(chunk_file_url.as_str()).await?;
            let chunk_file_path = join4(
                &repo_name,
                &INDEX_DIR_NAME,
                &CHUNK_DIR_NAME,
                &set_extension(
                    chunk_file,
                    "chunks",
                )?,
            )?;

            write_bytes(
                &chunk_file_path,
                &chunk_file_data,
                WriteMode::AlwaysCreate,
            )?;
            let chunks = chunk::load_from_file(&chunk_file_path)?;
            chunk::save_to_file(
                &chunk_file_path,
                &chunks,
                0,
                9,
                &repo_name,
                UpdateTfidf::Generate,
            )?;
        }

        let image_list_url = url.join("image-list")?;
        let image_list = request_json_file(image_list_url.as_str()).await?;
        let image_list = parse_vec_string(&image_list)?;

        for image_key in image_list.iter() {
            let image_url = url.join(&format!("image/{image_key}"))?;
            let image_desc_url = url.join(&format!("image-desc/{image_key}"))?;
            let image = request_binary_file(image_url.as_str()).await?;
            let image_desc = request_binary_file(image_desc_url.as_str()).await?;

            let image_path = join4(
                &repo_name,
                &INDEX_DIR_NAME,
                &IMAGE_DIR_NAME,
                &set_extension(
                    image_key,
                    "png",
                )?,
            )?;
            let image_desc_path = set_extension(&image_path, "json")?;

            write_bytes(
                &image_path,
                &image,
                WriteMode::AlwaysCreate,
            )?;
            write_bytes(
                &image_desc_path,
                &image_desc,
                WriteMode::AlwaysCreate,
            )?;
        }

        let meta_url = url.join("meta")?;
        let meta_json = request_binary_file(meta_url.as_str()).await?;
        let meta_path = join3(
            &repo_name,
            &INDEX_DIR_NAME,
            &METADATA_FILE_NAME,
        )?;
        write_bytes(
            &meta_path,
            &meta_json,
            WriteMode::AlwaysCreate,
        )?;

        Ok(())
    }
}

fn infer_repo_name_from_url(url: &str) -> String {
    todo!()
}

async fn request_json_file(url: &str) -> Result<JsonValue, Error> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if response.status().as_u16() != 200 {
        return Err(Error::CloneRequestError {
            code: response.status().as_u16(),
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
            code: response.status().as_u16(),
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