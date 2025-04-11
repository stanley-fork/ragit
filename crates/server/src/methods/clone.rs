use super::{HandleError, RawResponse, get_pool, handler};
use crate::models::{archive, repo};
use warp::reply::{Reply, json, with_header};

pub async fn get_archive_list(user: String, repo: String) -> Box<dyn Reply> {
    handler(get_archive_list_(user, repo).await)
}

async fn get_archive_list_(user: String, repo: String) -> RawResponse {
    let pool = get_pool().await;
    let session_id = repo::get_session_id(&user, &repo, pool).await.handle_error(404)?;
    let archive_list = match session_id {
        Some(session_id) => archive::get_list(&session_id, pool).await.handle_error(500)?,

        // It's not an error to clone an empty repository (github allows that)
        None => vec![],
    };
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

    // It's easy to count pushes: there's `finalize-push` api.
    // But clone operations use `/archive` and `/archive-list` apis, which are too general to count as a clone.
    // So we use a naive heuristic: we count downloading the first archive as a clone operation
    if archive::is_first_archive(&session_id, &archive_id, pool).await.handle_error(500)? {
        let repo_id = repo::get_id_by_name(&user, &repo, pool).await.handle_error(500)?;
        archive::increment_clone_count(repo_id, pool).await.handle_error(500)?;
    }

    Ok(Box::new(with_header(
        bytes,
        "Content-Type",
        "application/octet-stream",
    )))
}
