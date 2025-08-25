use super::{HandleError, RawResponse, check_secure_path, get_or, get_pool, handler};
use crate::CONFIG;
use crate::models::file::{FileDetail, FileSimple, FileType};
use crate::models::repo::{self, RepoCreation, RepoOperation};
use crate::models::user;
use crate::utils::get_rag_path;
use ragit::{
    Index,
    LoadMode,
    Uid,
    UidQueryConfig,
    into_multi_modal_contents,
};
use ragit_fs::{
    exists,
    join,
    join3,
    read_string,
    set_extension,
};
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use warp::http::StatusCode;
use warp::reply::{Reply, json, with_header, with_status};

// I set `body: Value` not `body: RepoCreation` because it gives a better error message for invalid schemas.
// It runs `rag init` on disk.
pub async fn create_repo(user: String, body: Value, api_key: Option<String>) -> Box<dyn Reply> {
    handler(create_repo_(user, body, api_key).await)
}

async fn create_repo_(user: String, body: Value, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo = serde_json::from_value::<RepoCreation>(body).handle_error(400)?;
    user::check_auth(&user, api_key, pool).await.handle_error(500)?.handle_error(403)?;
    repo.validate().handle_error(400)?;
    (!repo::check_existence(&user, &repo.name, pool).await.handle_error(500)?).handle_error(400)?;
    let repo_id = repo::create_and_return_id(&user, &repo, pool).await.handle_error(500)?;
    let config = CONFIG.get().handle_error(500)?;
    let index_path = join3(
        &config.repo_data_dir,
        &user,
        &repo.name,
    ).handle_error(400)?;
    Index::new(index_path).handle_error(500)?;

    Ok(Box::new(json(&repo_id)))
}

pub async fn get_index(user: String, repo: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_index_(user, repo, api_key).await)
}

async fn get_index_(user: String, repo: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;
    let index_path = join(&rag_path, "index.json").handle_error(400)?;
    let j = read_string(&index_path).handle_error(404)?;

    Ok(Box::new(with_header(
        j,
        "Content-Type",
        "application/json",
    )))
}

pub async fn get_uid(user: String, repo: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_uid_(user, repo, api_key).await)
}

async fn get_uid_(user: String, repo: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let config = CONFIG.get().handle_error(500)?;
    let rag_path = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;

    let uid = if exists(&rag_path) {
        let mut index = Index::load(rag_path, LoadMode::OnlyJson).handle_error(404)?;
        index.calculate_and_save_uid().handle_error(500)?
    }

    // when a repository is created via api, but nothing's pushed
    else {
        // uid of an empty knowledge-base
        Uid::new_knowledge_base(&[])
    };

    Ok(Box::new(with_header(
        uid.to_string(),
        "Content-Type",
        "text/plain; charset=utf-8",
    )))
}

pub async fn get_config(user: String, repo: String, config: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_config_(user, repo, config, api_key).await)
}

async fn get_config_(user: String, repo: String, config: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
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

pub async fn get_prompt(user: String, repo: String, prompt: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_prompt_(user, repo, prompt, api_key).await)
}

async fn get_prompt_(user: String, repo: String, prompt: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
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

pub async fn get_content(user: String, repo: String, uid: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_content_(user, repo, uid, api_key).await)
}

async fn get_content_(user: String, repo: String, uid: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let config = CONFIG.get().handle_error(500)?;
    let rag_path = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;
    let index = Index::load(rag_path, LoadMode::OnlyJson).handle_error(404)?;
    let query = index.uid_query(&[uid.clone()], UidQueryConfig::new().no_query_history()).handle_error(400)?;

    let data = if query.has_multiple_matches() {
        return Err((400, format!("There are multiple file/chunk that match `{uid}`.")));
    }

    else if let Some(uid) = query.get_chunk_uid() {
        let chunk = index.get_chunk_by_uid(uid).handle_error(500)?;
        into_multi_modal_contents(&chunk.data, &chunk.images)
    }

    else if let Some((_, uid)) = query.get_processed_file() {
        let chunk = index.get_merged_chunk_of_file(uid).handle_error(500)?;
        chunk.raw_data
    }

    else {
        return Err((404, format!("There's no file/chunk that matches `{uid}`")));
    };

    Ok(Box::new(json(&data)))
}

pub async fn get_cat_file(user: String, repo: String, uid: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_cat_file_(user, repo, uid, api_key).await)
}

async fn get_cat_file_(user: String, repo: String, uid: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let config = CONFIG.get().handle_error(500)?;
    let rag_path = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;
    let index = Index::load(rag_path, LoadMode::OnlyJson).handle_error(404)?;
    let query = index.uid_query(&[uid.clone()], UidQueryConfig::new().no_query_history()).handle_error(400)?;

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
        let chunk = index.get_merged_chunk_of_file(uid).handle_error(500)?;
        Ok(Box::new(with_header(
            chunk.human_data,
            "Content-Type",
            "text/plain; charset=utf-8",
        )))
    }

    else {
        Err((404, format!("There's no file/chunk that matches `{uid}`")))
    }
}

pub async fn get_meta(user: String, repo: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_meta_(user, repo, api_key).await)
}

async fn get_meta_(user: String, repo: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
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

pub async fn get_meta_by_key(user: String, repo: String, key: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_meta_by_key_(user, repo, key, api_key).await)
}

async fn get_meta_by_key_(user: String, repo: String, key: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;

    if !exists(&rag_path) {
        return Err((404, format!("No such repo: `{user}/{repo}`")));
    }

    let meta_path = join(&rag_path, "meta.json").handle_error(400)?;

    // NOTE: a `.ragit/` may or may not have `meta.json`
    let meta_json = read_string(&meta_path).unwrap_or(String::from("{}"));
    let meta_json = serde_json::from_str::<HashMap<String, String>>(&meta_json).handle_error(500)?;

    Ok(Box::new(with_header(
        serde_json::to_string(&meta_json.get(&key)).handle_error(500)?,
        "Content-Type",
        "application/json",
    )))
}

pub async fn get_file_content(user: String, repo: String, query: HashMap<String, String>, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_file_content_(user, repo, query, api_key).await)
}

async fn get_file_content_(user: String, repo: String, query: HashMap<String, String>, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;

    let limit = get_or(&query, "limit", 100);
    let mut offset = get_or(&query, "offset", 0);
    let mut path = get_or(&query, "path", String::from("/"));
    path = ragit_fs::normalize(&path).handle_error(400)?;

    if path.starts_with("/") {
        path = path.get(1..).unwrap().to_string();
    }

    let config = CONFIG.get().handle_error(500)?;
    let rag_path = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;
    let index = Index::load(rag_path, LoadMode::OnlyJson).handle_error(404)?;

    // It's a file
    let result = if let Some(uid) = index.processed_files.get(&path) {
        let chunk = index.get_merged_chunk_of_file(*uid).handle_error(500)?;
        let chunk_uids = index.get_chunks_of_file(*uid).handle_error(500)?;

        FileDetail {
            r#type: FileType::File,
            content: Some(chunk.raw_data),
            uid: Some(uid.to_string()),
            path: path.to_string(),
            chunks: Some(chunk_uids.iter().map(|chunk| chunk.to_string()).collect()),
            children: None,
        }
    }

    // otherwise, it's a directory
    else {
        // Ragit only tracks files, there's no concept of "directories" in ragit. So it tries
        // to infer the directory structures from the file paths.
        if !path.ends_with("/") {
            path = format!("{path}/");
        }

        if path == "/" {
            path = String::new();
        }

        let path_re = path
            .replace("\\", "\\\\")
            .replace("|", "\\|")
            .replace("(", "\\(")
            .replace(")", "\\)")
            .replace("{", "\\{")
            .replace("}", "\\}")
            .replace("[", "\\[")
            .replace("]", "\\]")
            .replace(".", "\\.")
            .replace("+", "\\+")
            .replace("*", "\\*")
            .replace("?", "\\?")
            .replace("^", "\\^")
            .replace("$", "\\$");
        let path_re = Regex::new(&format!("^{path_re}([^/]+)(/.+)?$")).handle_error(400)?;
        let mut processed_files = index.processed_files.keys().collect::<Vec<_>>();
        processed_files.sort();
        let mut children = vec![];
        let mut dir_set = HashSet::new();

        for file in processed_files.iter() {
            if let Some(cap) = path_re.captures(file) {
                if offset > 0 {
                    offset -= 1;
                    continue;
                }

                let child_name = cap.get(1).unwrap().as_str();
                let is_dir = cap.get(2).is_some();

                if is_dir {
                    if dir_set.contains(child_name) {
                        continue;
                    }

                    dir_set.insert(child_name.to_string());
                    children.push(
                        FileSimple {
                            r#type: FileType::Directory,
                            path: format!("{path}{child_name}/"),
                        }
                    );
                }

                else {
                    children.push(
                        FileSimple {
                            r#type: FileType::File,
                            path: format!("{path}{child_name}"),
                        }
                    );
                }

                if children.len() >= limit {
                    break;
                }
            }
        }

        if children.is_empty() {
            return Err((404, format!("No such dir: {path}")));
        }

        FileDetail {
            r#type: FileType::Directory,
            content: None,
            uid: None,
            path: path.to_string(),
            chunks: None,
            children: Some(children),
        }
    };

    Ok(Box::new(json(&result)))
}

pub async fn get_version(user: String, repo: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_version_(user, repo, api_key).await)
}

async fn get_version_(user: String, repo: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
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

pub async fn post_build_search_index(user: String, repo: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(post_build_search_index_(user, repo, api_key).await)
}

async fn post_build_search_index_(user: String, repo: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Write, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let config = CONFIG.get().handle_error(500)?;
    let rag_path = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;

    // we don't have to check whether the search index is already built.
    // if so, `index.build_ii` will return early
    let mut index = Index::load(rag_path, LoadMode::OnlyJson).handle_error(404)?;
    index.build_ii(true /* quiet */).handle_error(500)?;

    repo::update_search_index_build_time(repo_id, pool).await.handle_error(500)?;
    Ok(Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    )))
}
