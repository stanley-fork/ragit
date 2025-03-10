use super::{HandleError, RawResponse, handler};
use crate::utils::get_rag_path;
use ragit_fs::{
    basename,
    exists,
    join,
    join3,
    read_bytes,
    read_dir,
};
use warp::reply::{Reply, json, with_header};

pub fn get_archive_list(user: String, repo: String) -> Box<dyn Reply> {
    handler(get_archive_list_(user, repo))
}

fn get_archive_list_(user: String, repo: String) -> RawResponse {
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;

    if !exists(&rag_path) {
        return Err((404, format!("No such repo: `{user}/{repo}`")));
    }

    let archive_path = join(&rag_path, "archives").handle_error(404)?;
    let archives: Vec<String> = read_dir(&archive_path, true).unwrap_or(vec![]).iter().map(
        |f| basename(&f).unwrap_or(String::new())
    ).filter(
        |f| !f.is_empty()
    ).collect();
    Ok(Box::new(json(&archives)))
}

pub fn get_archive(user: String, repo: String, archive_key: String) -> Box<dyn Reply> {
    handler(get_archive_(user, repo, archive_key))
}

fn get_archive_(user: String, repo: String, archive_key: String) -> RawResponse {
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;
    let archive_path = join3(&rag_path, "archives", &archive_key).handle_error(400)?;
    let bytes = read_bytes(&archive_path).handle_error(404)?;

    Ok(Box::new(with_header(
        bytes,
        "Content-Type",
        "application/octet-stream",
    )))
}
