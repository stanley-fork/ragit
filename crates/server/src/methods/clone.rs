use super::{HandleError, RawResponse, get_pool, handler};
use crate::models::{archive, repo};
use warp::reply::{Reply, json, with_header};

pub async fn get_archive_list(user: String, repo: String) -> Box<dyn Reply> {
    handler(get_archive_list_(user, repo).await)
}

async fn get_archive_list_(user: String, repo: String) -> RawResponse {
    let pool = get_pool().await;
    let session_id = repo::get_session_id(&user, &repo, pool).await.handle_error(404)?;
    let Some(session_id) = session_id else {
        // TODO: I want to provide this info to client
        return Err((400, format!("Nothing's pushed to `{user}/{repo}` yet!")));
    };

    let archive_list = archive::get_list(&session_id, pool).await.handle_error(500)?;
    Ok(Box::new(json(&archive_list)))
}

pub async fn get_archive(user: String, repo: String, archive_id: String) -> Box<dyn Reply> {
    handler(get_archive_(user, repo, archive_id).await)
}

async fn get_archive_(user: String, repo: String, archive_id: String) -> RawResponse {
    let pool = get_pool().await;
    let session_id = repo::get_session_id(&user, &repo, pool).await.handle_error(404)?;
    let Some(session_id) = session_id else {
        return Err((400, format!("Nothing's pushed to `{user}/{repo}` yet!")));
    };

    let bytes = archive::get_archive(&session_id, &archive_id, pool).await.handle_error(404)?;

    Ok(Box::new(with_header(
        bytes,
        "Content-Type",
        "application/octet-stream",
    )))
}
