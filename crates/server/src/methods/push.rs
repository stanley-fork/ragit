use super::{HandleError, RawResponse, get_pool, handler};
use bytes::Bytes;
use crate::CONFIG;
use crate::models::{archive, repo};
use crate::models::archive::PushResult;
use crate::models::repo::RepoOperation;
use crate::utils::fetch_form_data;
use ragit::Index;
use ragit_fs::{
    WriteMode,
    create_dir_all,
    join,
    join3,
    read_dir,
    remove_dir_all,
    write_bytes,
    write_log,
};
use warp::filters::multipart::FormData;
use warp::http::StatusCode;
use warp::reply::{Reply, with_header, with_status};

pub async fn post_begin_push(user: String, repo: String, api_key: Option<String>) -> Box<dyn Reply> {
    handler(post_begin_push_(user, repo, api_key).await)
}

async fn post_begin_push_(user: String, repo: String, api_key: Option<String>) -> RawResponse {
    let config = CONFIG.get().handle_error(500)?;
    let pool = get_pool().await;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    repo::check_auth(repo_id, RepoOperation::Push, api_key, pool).await.handle_error(500)?.handle_error(404)?;
    let session_id = archive::create_new_session(repo_id, pool).await.handle_error(500)?;

    create_dir_all(
        &join(
            &config.push_session_dir,
            &session_id,
        ).handle_error(500)?,
    ).handle_error(500)?;

    Ok(Box::new(with_header(
        session_id,
        "Content-Type",
        "text/plain; charset=utf-8",
    )))
}

pub async fn post_archive(user: String, repo: String, form: FormData) -> Box<dyn Reply> {
    handler(post_archive_(user, repo, form).await)
}

async fn post_archive_(_user: String, _repo: String, form: FormData) -> RawResponse {
    let config = CONFIG.get().handle_error(500)?;
    let pool = get_pool().await;
    let form = fetch_form_data(form).await.handle_error(400)?;
    let session_id = form.get("session-id").ok_or_else(|| "session-id not found").handle_error(400)?;
    let session_id = String::from_utf8_lossy(session_id).to_string();
    let archive_id = form.get("archive-id").ok_or_else(|| "archive-id not found").handle_error(400)?;
    let archive_id = String::from_utf8(archive_id.to_vec()).handle_error(400)?;
    let archive = form.get("archive").ok_or_else(|| "archive not found").handle_error(400)?;
    archive::add_archive(&session_id, &archive_id, &archive, pool).await.handle_error(500)?;

    let path = join3(
        &config.push_session_dir,
        &session_id,
        &archive_id,
    ).handle_error(400)?;

    write_bytes(
        &path,
        &archive,
        WriteMode::AlwaysCreate,
    ).handle_error(500)?;

    Ok(Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    )))
}

pub async fn post_finalize_push(user: String, repo: String, body: Bytes) -> Box<dyn Reply> {
    handler(post_finalize_push_(user, repo, body).await)
}

async fn post_finalize_push_(user: String, repo: String, body: Bytes) -> RawResponse {
    let config = CONFIG.get().handle_error(500)?;
    let pool = get_pool().await;
    let session_id = String::from_utf8(body.into_iter().collect::<Vec<u8>>()).handle_error(400)?;
    let archives_at = join(
        &config.push_session_dir,
        &session_id,
    ).handle_error(400)?;
    let repo_id = repo::get_id(&user, &repo, pool).await.handle_error(404)?;
    let archives = read_dir(&archives_at, false).handle_error(404)?;
    let root_dir = join3(
        &config.repo_data_dir,
        &user,
        &repo,
    ).handle_error(400)?;

    write_log(
        "post_finalize_push",
        &format!("start extracting archive at `{root_dir}`"),
    );

    let push_result = Index::extract_archive(
        &root_dir,
        archives.clone(),
        4,

        // TODO: is it okay to force-extract? if there's an error, it might lose the original data
        true,
        true,  // quiet
    );
    let push_result = match push_result {
        Ok(_) => PushResult::Completed,
        Err(e) => {
            write_log(
                "post_finalize_push",
                &format!("Error at `Index::extract_archive`: {e:?}"),
            );
            PushResult::Failed
        },
    };
    archive::finalize_push(
        repo_id,
        &session_id,
        push_result,
        pool,
    ).await.handle_error(500)?;
    remove_dir_all(
        &join(
            &config.push_session_dir,
            &session_id,
        ).handle_error(500)?,
    ).handle_error(500)?;

    Ok(Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    )))
}
