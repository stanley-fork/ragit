use super::{HandleError, RawResponse, get_pool, handler};
use crate::CONFIG;
use crate::models::repo::{self, RepoOperation};
use ragit::{Index, LoadMode};
use ragit_fs::{join3, write_log};
use std::collections::HashMap;
use std::str::FromStr;
use warp::reply::{Reply, json};

// TODO: it has to be tfidf search on chunks/files/images, with additional search-by-feature (what it implements now)
pub async fn search(user: String, repo: String, query: HashMap<String, String>, api_key: Option<String>) -> Box<dyn Reply> {
    handler(search_(user, repo, query, api_key).await)
}

pub async fn search_(user: String, repo: String, query: HashMap<String, String>, api_key: Option<String>) -> RawResponse {
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

    write_log(
        "search",
        &format!("search({user:?}, {repo:?}, {query:?})"),
    );

    let limit = get_or(&query, "limit", 100);
    let file = get_or(&query, "file", String::new());

    let mut chunk_uids = if file == "" {
        index.get_all_chunk_uids().handle_error(500)?
    } else {
        let file_uid = match index.processed_files.get(&file) {
            Some(uid) => *uid,
            None => {
                return Err((404, format!("File not found: `{file}`")));
            },
        };

        index.get_chunks_of_file(file_uid).handle_error(404)?
    };

    if chunk_uids.len() > limit {
        chunk_uids = chunk_uids[..limit].to_vec();
    }

    let mut chunks = Vec::with_capacity(chunk_uids.len());

    for chunk_uid in chunk_uids.iter() {
        chunks.push(index.get_chunk_by_uid(*chunk_uid).handle_error(500)?);
    }

    Ok(Box::new(json(&chunks)))
}

fn get_or<T: FromStr>(query: &HashMap<String, String>, key: &str, default_value: T) -> T {
    match query.get(key) {
        Some(v) => match v.parse::<T>() {
            Ok(v) => v,
            Err(_) => default_value,
        },
        None => default_value,
    }
}
