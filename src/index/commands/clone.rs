use super::Index;
use crate::{INDEX_DIR_NAME, chunk};
use crate::error::Error;
use crate::index::commands::meta::METADATA_FILE_NAME;
use crate::uid::Uid;
use ragit_fs::{
    WriteMode,
    create_dir,
    create_dir_all,
    exists,
    join3,
    parent,
    remove_dir_all,
    set_extension,
    write_bytes,
};
use reqwest::Url;
use serde_json::Value;

pub struct CloneResult {
    url: String,
    image_total: usize,
    image_count: usize,
    chunk_total: usize,
    chunk_count: usize,
}

impl Index {
    pub async fn clone(url: String, repo_name: Option<String>) -> Result<CloneResult, Error> {
        let repo_name = repo_name.unwrap_or_else(|| infer_repo_name_from_url(&url));
        create_dir(&repo_name)?;

        match Index::clone_worker(url, repo_name.clone()).await {
            Ok(result) => Ok(result),
            Err(e) => {
                remove_dir_all(&repo_name)?;
                Err(e)
            },
        }
    }

    // TODO: configs, prompts
    async fn clone_worker(mut url: String, repo_name: String) -> Result<CloneResult, Error> {
        if !url.ends_with("/") {
            url = format!("{url}/");
        }

        let mut result = CloneResult {
            url: url.clone(),
            image_total: 0,
            image_count: 0,
            chunk_total: 0,
            chunk_count: 0,
        };

        let mut url = Url::parse(&url)?;
        url.set_port(Some(41127)).map_err(|_| Error::CloneRequestError {
            code: None,
            url: url.as_str().into(),
        })?;
        let mut index = Index::new(repo_name.clone())?;

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

        // It has to download images before chunks because `chunk::save_to_file` requires
        // image descriptions.
        for prefix in 0..256 {
            let prefix_str = format!("{prefix:02x}");
            let image_list_url = url.join(&format!("image-list/{prefix_str}"))?;
            let image_list = request_json_file(image_list_url.as_str()).await?;
            let image_list = serde_json::from_value::<Vec<String>>(image_list)?;
            result.image_total += image_list.len();

            for image_uid_str in image_list.iter() {
                let image_url = url.join(&format!("image/{image_uid_str}"))?;
                let image_desc_url = url.join(&format!("image-desc/{image_uid_str}"))?;
                let image = request_binary_file(image_url.as_str()).await?;
                let image_desc = request_binary_file(image_desc_url.as_str()).await?;
                let image_uid = image_uid_str.parse::<Uid>()?;
                result.image_count += 1;
                Index::render_clone_dashboard(&result);
                let image_path = Index::get_image_path(
                    &index.root_dir,
                    image_uid,
                    "png",
                );
                let image_desc_path = set_extension(&image_path, "json")?;
                let parent = parent(&image_path)?;

                if !exists(&parent) {
                    create_dir_all(&parent)?;
                }

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
        }

        for prefix in 0..256 {
            let prefix_str = format!("{prefix:02x}");
            let chunk_list_url = url.join(&format!("chunk-list/{prefix_str}"))?;
            let chunk_list = request_json_file(chunk_list_url.as_str()).await?;
            let chunk_list = serde_json::from_value::<Vec<String>>(chunk_list)?;
            result.chunk_total += chunk_list.len();

            for chunk_uid_str in chunk_list.iter() {
                let chunk_url = url.join(&format!("chunk/{chunk_uid_str}"))?;
                let chunk_raw = request_binary_file(chunk_url.as_str()).await?;
                let chunk_uid = chunk_uid_str.parse::<Uid>()?;
                result.chunk_count += 1;
                Index::render_clone_dashboard(&result);
                let chunk_path = Index::get_chunk_path(
                    &index.root_dir,
                    chunk_uid,
                );
                let parent = parent(&chunk_path)?;

                if !exists(&parent) {
                    create_dir_all(&parent)?;
                }

                write_bytes(
                    &chunk_path,
                    &chunk_raw,
                    WriteMode::AlwaysCreate,
                )?;

                let chunk = chunk::load_from_file(&chunk_path)?;
                chunk::save_to_file(
                    &chunk_path,
                    &chunk,
                    0,
                    9,
                    &index.root_dir,
                )?;
            }
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
        index.repo_url = Some(url.to_string());
        index.save_to_file()?;
        Ok(result)
    }

    // TODO: erase lines instead of the entire screen
    fn render_clone_dashboard(result: &CloneResult) {
        clearscreen::clear().expect("failed to clear screen");
        println!("cloning {}...", result.url);
        println!("chunks: {}/{}", result.chunk_count, result.chunk_total);
        println!("images: {}/{}", result.image_count, result.image_total);
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

async fn request_json_file(url: &str) -> Result<Value, Error> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if response.status().as_u16() != 200 {
        return Err(Error::CloneRequestError {
            code: Some(response.status().as_u16()),
            url: url.to_string(),
        });
    }

    Ok(serde_json::from_str(&response.text().await?)?)
}
