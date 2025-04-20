use super::{HandleError, RawResponse, get_pool, handler};
use crate::models::repo::{self, RepoOperation};
use crate::utils::get_rag_path;
use ragit_fs::{
    exists,
    extension,
    file_name,
    join3,
    join4,
    read_bytes,
    read_dir,
    set_extension,
};
use warp::reply::{Reply, json, with_header};

pub async fn get_image_list(user: String, repo: String, prefix: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_image_list_(user, repo, prefix, api_key).await)
}

async fn get_image_list_(user: String, repo: String, prefix: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;

    if !exists(&rag_path) {
        return Err((404, format!("`{user}/{repo}` does not exist")));
    }

    let image_path = join3(
        &rag_path,
        "images",
        &prefix,
    ).handle_error(400)?;
    let images = read_dir(&image_path, false).unwrap_or(vec![]);

    Ok(Box::new(json(
        &images.iter().filter_map(
            |image| match extension(image) {
                Ok(Some(png)) if png == "png" => file_name(image).ok().map(|suffix| format!("{prefix}{suffix}")),
                _ => None,
            }
        ).collect::<Vec<String>>(),
    )))
}

pub async fn get_image(user: String, repo: String, uid: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_image_(user, repo, uid, api_key).await)
}

async fn get_image_(user: String, repo: String, uid: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let rag_path = get_rag_path(&user, &repo).handle_error(400)?;
    let prefix = uid.get(0..2).ok_or_else(|| format!("invalid uid: {uid}")).handle_error(400)?.to_string();
    let suffix = uid.get(2..).ok_or_else(|| format!("invalid uid: {uid}")).handle_error(400)?.to_string();
    let image_path = join4(
        &rag_path,
        "images",
        &prefix,
        &set_extension(&suffix, "png").handle_error(400)?,
    ).handle_error(400)?;
    let bytes = read_bytes(&image_path).handle_error(404)?;

    Ok(Box::new(with_header(
        bytes,
        "Content-Type",
        "image/png",
    )))
}

pub async fn get_image_desc(user: String, repo: String, uid: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(get_image_desc_(user, repo, uid, api_key).await)
}

async fn get_image_desc_(user: String, repo: String, uid: String, api_key: Option<String>) -> RawResponse {
    let pool = get_pool().await;
    let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Read, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let rag_path = get_rag_path(&user, &repo).handle_error(404)?;
    let prefix = uid.get(0..2).ok_or_else(|| format!("invalid uid: {uid}")).handle_error(400)?.to_string();
    let suffix = uid.get(2..).ok_or_else(|| format!("invalid uid: {uid}")).handle_error(400)?.to_string();
    let image_path = join4(
        &rag_path,
        "images",
        &prefix,
        &set_extension(&suffix, "json").handle_error(400)?,
    ).handle_error(400)?;
    let bytes = read_bytes(&image_path).handle_error(404)?;

    Ok(Box::new(with_header(
        bytes,
        "Content-Type",
        "application/json",
    )))
}
