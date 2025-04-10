use super::{HandleError, RawResponse, check_secure_path, handler};
use crate::CONFIG;
use crate::utils::get_rag_path;
use ragit::{Index, LoadMode, UidQueryConfig, merge_and_convert_chunks};
use ragit_fs::{
    exists,
    join,
    join3,
    read_string,
    set_extension,
};
use serde_json::Value;
use warp::http::StatusCode;
use warp::reply::{Reply, json, with_header, with_status};

pub fn get_index(user: String, repo: String) -> Box<dyn Reply> {
    handler(get_index_(user, repo))
}

fn get_index_(user: String, repo: String) -> RawResponse {
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;
    let index_path = join(&rag_path, "index.json").handle_error(400)?;
    let j = read_string(&index_path).handle_error(404)?;

    Ok(Box::new(with_header(
        j,
        "Content-Type",
        "application/json",
    )))
}

pub fn get_config(user: String, repo: String, config: String) -> Box<dyn Reply> {
    handler(get_config_(user, repo, config))
}

fn get_config_(user: String, repo: String, config: String) -> RawResponse {
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;
    check_secure_path(&config).handle_error(400)?;
    let config_path = join3(
        &rag_path,
        "configs",
        &set_extension(&config, "json").handle_error(400)?,
    ).handle_error(404)?;
    let j = read_string(&config_path).handle_error(404)?;

    Ok(Box::new(with_header(
        j,
        "Content-Type",
        "application/json",
    )))
}

pub fn get_prompt(user: String, repo: String, prompt: String) -> Box<dyn Reply> {
    handler(get_prompt_(user, repo, prompt))
}

fn get_prompt_(user: String, repo: String, prompt: String) -> RawResponse {
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;
    check_secure_path(&prompt).handle_error(400)?;
    let prompt_path = join3(
        &rag_path,
        "prompts",
        &set_extension(&prompt, "pdl").handle_error(400)?,
    ).handle_error(400)?;
    let p = read_string(&prompt_path).handle_error(404)?;

    Ok(Box::new(with_header(
        p,
        "Content-Type",
        "text/plain; charset=utf-8",
    )))
}

pub fn get_cat_file(user: String, repo: String, uid: String) -> Box<dyn Reply> {
    handler(get_cat_file_(user, repo, uid))
}

fn get_cat_file_(user: String, repo: String, uid: String) -> RawResponse {
    let config = CONFIG.get().handle_error(500)?;
    let rag_path = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;
    let index = Index::load(rag_path, LoadMode::OnlyJson).handle_error(404)?;
    let query = index.uid_query(&[uid.clone()], UidQueryConfig::new()).handle_error(400)?;

    if query.has_multiple_matches() {
        Err((400, format!("There are multiple file/chunk that match `{uid}`.")))
    }

    else if let Some(uid) = query.get_chunk_uid() {
        let chunk = index.get_chunk_by_uid(uid).handle_error(500)?;

        Ok(Box::new(with_header(
            chunk.data,
            "Content-Type",
            "text/plain; charset=utf-8",
        )))
    }

    else if let Some((_, uid)) = query.get_processed_file() {
        let chunk_uids = index.get_chunks_of_file(uid).handle_error(500)?;
        let mut chunks = Vec::with_capacity(chunk_uids.len());

        for chunk_uid in chunk_uids {
            chunks.push(index.get_chunk_by_uid(chunk_uid).handle_error(500)?);
        }

        chunks.sort_by_key(|chunk| chunk.source.sortable_string());
        let chunks = merge_and_convert_chunks(&index, chunks, true /* render_image */).handle_error(500)?;

        let result = match chunks.len() {
            0 => String::new(),
            1 => chunks[0].data.clone(),
            _ => {
                return Err((500, format!("`index.get_chunks_of_file({uid})` returned chunks from different files.")));
            },
        };

        Ok(Box::new(with_header(
            result,
            "Content-Type",
            "text/plain; charset=utf-8",
        )))
    }

    else {
        Err((404, format!("There's no file/chunk that matches `{uid}`")))
    }
}

pub fn get_meta(user: String, repo: String) -> Box<dyn Reply> {
    handler(get_meta_(user, repo))
}

fn get_meta_(user: String, repo: String) -> RawResponse {
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;

    if !exists(&rag_path) {
        return Err((404, format!("No such repo: `{user}/{repo}`")));
    }

    let meta_path = join(&rag_path, "meta.json").handle_error(400)?;

    // NOTE: a `.ragit/` may or may not have `meta.json`
    let meta_json = read_string(&meta_path).unwrap_or(String::from("{}"));

    Ok(Box::new(with_header(
        meta_json,
        "Content-Type",
        "application/json",
    )))
}

// TODO: it has to be a search endpoint for files
//       we have to implement github-like file viewer
pub fn get_file_list(user: String, repo: String) -> Box<dyn Reply> {
    handler(get_file_list_(user, repo))
}

fn get_file_list_(user: String, repo: String) -> RawResponse {
    let config = CONFIG.get().handle_error(500)?;
    let rag_path = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;
    let index = Index::load(rag_path, LoadMode::OnlyJson).handle_error(404)?;
    Ok(Box::new(json(&index.processed_files.keys().collect::<Vec<_>>())))
}

pub fn get_version(user: String, repo: String) -> Box<dyn Reply> {
    handler(get_version_(user, repo))
}

fn get_version_(user: String, repo: String) -> RawResponse {
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;
    let index_path = join(&rag_path, "index.json").handle_error(400)?;
    let index_json = read_string(&index_path).handle_error(404)?;
    let index = serde_json::from_str::<Value>(&index_json).handle_error(500)?;

    let (code, error) = match index {
        Value::Object(obj) => match obj.get("ragit_version") {
            Some(v) => match v.as_str() {
                Some(v) => {
                    return Ok(Box::new(with_header(
                        v.to_string(),
                        "Content-Type",
                        "text/plain; charset=utf-8",
                    )));
                },
                None => (500, format!("`{v:?}` is not a valid string")),
            },
            None => (500, format!("`{index_path}` has no `ragit_version` field")),
        },
        index => (500, format!("`{index:?}` is not a valid index")),
    };

    Err((code, error))
}

pub fn post_ii_build(user: String, repo: String) -> Box<dyn Reply> {
    handler(post_ii_build_(user, repo))
}

fn post_ii_build_(user: String, repo: String) -> RawResponse {
    let config = CONFIG.get().handle_error(500)?;
    let rag_path = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;
    let mut index = Index::load(rag_path, LoadMode::OnlyJson).handle_error(404)?;
    index.build_ii(true /* quiet */).handle_error(500)?;
    Ok(Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    )))
}
