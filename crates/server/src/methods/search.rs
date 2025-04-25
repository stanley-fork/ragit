use super::{HandleError, RawResponse, get_pool, handler};
use crate::CONFIG;
use crate::models::chunk::ChunkDetail;
use crate::models::repo::{self, RepoOperation};
use ragit::{
    ChunkSource,
    Index,
    Keywords,
    LoadMode,
    TfidfResult,
    UidQueryConfig,
};
use ragit_fs::{join3, write_log};
use std::collections::HashMap;
use std::str::FromStr;
use warp::reply::{Reply, json};

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

    let limit = get_or(&query, "limit", 50);
    let offset = get_or(&query, "offset", 0);

    // It uses a simple hack to search by file/dir.
    // If the query is "foo/bar", it searches for files whose
    // 1) path is exactly "foo/bar" or 2) path starts with "foo/bar/",
    // but doesn't search for paths that start with "foo/bar".
    // It makes sense because the paths are all normalized.
    let file = get_or(&query, "file", String::new());
    let dir = if !file.ends_with("/") { format!("{file}/") } else { file.clone() };

    let uid_prefix = get_or(&query, "uid", String::new());
    let keywords = get_or(&query, "query", String::new());
    let tokenized_keywords = Keywords::from_raw(vec![keywords.to_string()]);

    // if `keywords` are given, it sorts the result by tfidf score
    // otherwise, it sorts the chunks by `chunk.sortable_string()`,
    // which is usually a name of a file the chunk belongs to.
    let mut has_to_sort_by_file = true;

    let mut has_to_search_by_keywords = keywords != "";
    let mut is_limit_applied = false;
    let mut is_offset_applied = false;

    let mut chunks = if uid_prefix != "" {
        let query_result = index
            .uid_query(&[uid_prefix.to_string()], UidQueryConfig::new().chunk_only())
            .handle_error(500)?
            .get_chunk_uids();
        let mut chunks = Vec::with_capacity(query_result.len());

        for uid in query_result.iter() {
            chunks.push(index.get_chunk_by_uid(*uid).handle_error(500)?);
        }

        if file != "" {
            chunks = chunks.into_iter().filter(
                |chunk| match &chunk.source {
                    ChunkSource::File { path, .. } => path == &file || path.starts_with(&dir),
                    _ => false,
                }
            ).collect();
        }

        chunks
    }

    else if file != "" {
        let chunk_uids = match index.processed_files.get(&file) {
            Some(uid) => index.get_chunks_of_file(*uid).handle_error(500)?,
            None => {
                let mut chunk_uids = vec![];

                for (file, uid) in index.processed_files.iter() {
                    if file.starts_with(&dir) {
                        chunk_uids.append(&mut index.get_chunks_of_file(*uid).handle_error(500)?);
                    }
                }

                chunk_uids
            },
        };
        let mut chunks = Vec::with_capacity(chunk_uids.len());

        for uid in chunk_uids.iter() {
            chunks.push(index.get_chunk_by_uid(*uid).handle_error(500)?);
        }

        chunks
    }

    else if keywords != "" {
        // TODO: impl `offset` parameter for `run_tfidf`
        let result = index.run_tfidf(
            tokenized_keywords.clone(),
            limit + offset,
        ).handle_error(500)?;
        has_to_search_by_keywords = false;
        has_to_sort_by_file = false;
        let mut chunks = Vec::with_capacity(limit.min(result.len()));

        for (i, TfidfResult { id: uid, score: _ }) in result.iter().enumerate() {
            if i >= offset {
                chunks.push(index.get_chunk_by_uid(*uid).handle_error(500)?);
            }
        }

        is_offset_applied = true;
        is_limit_applied = true;
        chunks
    }

    // No condition at all: sort all the chunks by `sortable_string`, and apply limit & offset
    // Simple hack: `sortable_string` sorts chunks by file name
    // TODO: what if a chunk is not from a file?
    else {
        let mut processed_files = index.processed_files.iter().collect::<Vec<_>>();
        processed_files.sort_by_key(|(file, _)| file.to_string());
        let mut chunk_uids = Vec::with_capacity(limit + offset);

        for (_, uid) in processed_files.iter() {
            chunk_uids.append(&mut index.get_chunks_of_file(**uid).handle_error(500)?);

            if chunk_uids.len() > limit + offset {
                break;
            }
        }

        let mut chunks = Vec::with_capacity(chunk_uids.len());

        for chunk_uid in chunk_uids.iter() {
            chunks.push(index.get_chunk_by_uid(*chunk_uid).handle_error(500)?);
        }

        chunks
    };

    if has_to_search_by_keywords {
        let result = index.run_tfidf_on(
            &chunks.iter().map(|chunk| chunk.uid).collect::<Vec<_>>(),
            tokenized_keywords,
            limit + offset,
        ).handle_error(500)?;
        let mut chunks_ = Vec::with_capacity(limit.min(result.len()));

        for (i, TfidfResult { id: uid, score: _ }) in result.iter().enumerate() {
            if i >= offset {
                chunks_.push(index.get_chunk_by_uid(*uid).handle_error(500)?);
            }
        }

        has_to_sort_by_file = false;
        is_offset_applied = true;
        is_limit_applied = true;
        chunks = chunks_;
    }

    if has_to_sort_by_file {
        chunks.sort_by_key(|chunk| chunk.sortable_string());
    }

    if !is_offset_applied {
        if offset >= chunks.len() {
            chunks = vec![];
        }

        else {
            chunks = chunks[offset..].to_vec();
        }
    }

    if !is_limit_applied && chunks.len() > limit {
        chunks = chunks[..limit].to_vec();
    }

    let chunks = chunks.into_iter().map(|c| ChunkDetail::from(c)).collect::<Vec<_>>();
    Ok(Box::new(json(&chunks)))
}

fn get_or<T: FromStr>(query: &HashMap<String, String>, key: &str, default_value: T) -> T {
    match query.get(key) {
        // many clients use an empty string to represent a null value
        Some(v) if v.is_empty() => default_value,

        Some(v) => match v.parse::<T>() {
            Ok(v) => v,
            Err(_) => default_value,
        },
        None => default_value,
    }
}
