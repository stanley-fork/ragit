use super::{HandleError, RawResponse, handler};
use bytes::Bytes;
use chrono::Local;
use crate::error::Error;
use crate::models::Chat;
use crate::utils::{decode_base64, fetch_form_data, get_rag_path};
use ragit::{Index, LoadMode, QueryTurn};
use ragit_fs::{
    FileError,
    WriteMode,
    create_dir_all,
    exists,
    file_name,
    join,
    join3,
    join4,
    parent,
    read_dir,
    remove_dir_all,
    rename,
    set_extension,
    write_bytes,
    write_string,
    write_log,
};
use std::collections::HashMap;
use warp::Reply;
use warp::filters::multipart::FormData;
use warp::http::StatusCode;
use warp::reply::{json, with_header, with_status};

#[derive(Copy, Clone)]
struct Session {
    id: u128,
    last_updated: i64,
}

// TODO: any better way for global vars?
const SESSION_POOL_SIZE: usize = 64;
static mut SESSIONS: [Option<Session>; SESSION_POOL_SIZE] = [None; SESSION_POOL_SIZE];

// TODO: some handlers return 500, some 404 or 503, but I'm not sure which one is correct in which cases

pub fn post_begin_push(user: String, repo: String, auth_info: Option<String>) -> Box<dyn Reply> {
    handler(post_begin_push_(user, repo, auth_info))
}

fn post_begin_push_(user: String, repo: String, auth_info: Option<String>) -> RawResponse {
    let session_id = rand::random::<u128>();
    let root_dir = get_rag_path(&user, &repo).handle_error(404)?;
    let mut auth_parsed: Option<(String, Option<String>)> = None;

    if let Some(auth_) = auth_info {
        if let Some(auth_) = auth_.get(6..) {  // `Basic {auth_}`
            if let Ok(auth_) = decode_base64(&auth_) {
                let auth_ = String::from_utf8_lossy(&auth_).to_string();
                let splitted = auth_.split(":").collect::<Vec<_>>();

                match (splitted.get(0), splitted.get(1)) {
                    (Some(username), None) => { auth_parsed = Some((username.to_string(), None)); },
                    (Some(username), Some(password)) if !password.is_empty() => { auth_parsed = Some((username.to_string(), Some(password.to_string()))); },
                    (Some(username), Some(_)) => { auth_parsed = Some((username.to_string(), None)); },
                    (None, _) => {},
                }
            }
        }
    };

    if !auth(&user, &repo, &auth_parsed) {
        return Err((403, format!("unauthorized access to `{user}/{repo}`")));
    }

    if !exists(&root_dir) {
        create_dir_all(&parent(&root_dir).handle_error(500)?).handle_error(500)?;
    }

    try_register_session(session_id).handle_error(503)?;
    let session_id = format!("{session_id:032x}");
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
    let form = fetch_form_data(form).await.handle_error(400)?;
    let session_id = form.get("session-id").ok_or_else(|| "session-id not found").handle_error(400)?;
    let session_id = u128::from_str_radix(&String::from_utf8_lossy(session_id), 16).handle_error(400)?;
    let archive_id = form.get("archive-id").ok_or_else(|| "archive-id not found").handle_error(400)?;
    let archive_id = String::from_utf8(archive_id.to_vec()).handle_error(400)?;
    let archive = form.get("archive").ok_or_else(|| "archive not found").handle_error(400)?;

    try_update_timestamp(session_id).handle_error(404)?;

    let path = join3(
        "./session",
        &format!("{session_id:032x}"),
        &archive_id,
    ).handle_error(500)?;

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

pub fn post_finalize_push(user: String, repo: String, body: Bytes) -> Box<dyn Reply> {
    handler(post_finalize_push_(user, repo, body))
}

fn post_finalize_push_(user: String, repo: String, body: Bytes) -> RawResponse {
    let session_id = String::from_utf8(body.into_iter().collect::<Vec<u8>>()).handle_error(400)?;
    let session_id = u128::from_str_radix(&session_id, 16).handle_error(400)?;
    let root_dir = parent(&get_rag_path(&user, &repo).handle_error(404)?).handle_error(404)?;
    let archives_at = join(
        "./session",
        &format!("{session_id:032x}"),
    ).handle_error(404)?;
    let archives = read_dir(&archives_at, false).handle_error(404)?;

    write_log(
        "post_finalize_push",
        &format!("start extracting archive at `{root_dir}`"),
    );

    Index::extract_archive(
        &root_dir,
        archives.clone(),
        4,

        // TODO: is it okay to force-extract? if there's an error, it might lose the original data
        true,
        true,  // quiet
    ).handle_error(500)?;

    if !exists(&join3(
        &root_dir,
        ".ragit",
        "archives",
    ).handle_error(500)?) {
        create_dir_all(&join3(
            &root_dir,
            ".ragit",
            "archives",
        ).handle_error(500)?).handle_error(500)?;
    }

    for archive in archives.iter() {
        rename(archive, &join4(
            &root_dir,
            ".ragit",
            "archives",
            &file_name(archive).handle_error(500)?,
        ).handle_error(500)?).handle_error(500)?;
    }

    try_unregister_session(session_id).handle_error(500)?;

    Ok(Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    )))
}

pub async fn post_chat(user: String, repo: String, chat_id: String, form: HashMap<String, Vec<u8>>) -> Box<dyn Reply> {
    handler(post_chat_(user, repo, chat_id, form).await)
}

async fn post_chat_(user: String, repo: String, chat_id: String, form: HashMap<String, Vec<u8>>) -> RawResponse {
    let query = match form.get("query") {
        Some(query) => String::from_utf8_lossy(query).to_string(),
        None => {
            return Err((400, String::from("`query` field is missing")));
        },
    };
    let model = match form.get("model") {
        Some(model) => Some(String::from_utf8_lossy(model).to_string()),
        None => None,
    };
    let index_at = join3(
        "data",
        &user,
        &repo,
    ).handle_error(400)?;
    let chat_at = join3(
        &index_at,
        "chats",
        &set_extension(&chat_id, "json").handle_error(400)?,
    ).handle_error(400)?;

    let mut chat = Chat::load_from_file(&chat_at).handle_error(404)?;
    let mut index = Index::load(index_at, LoadMode::OnlyJson).handle_error(500)?;

    if let Some(model) = model {
        index.api_config.model = model;
    }

    let response = index.query(&query, chat.history.clone()).await.handle_error(500)?;
    chat.history.push(QueryTurn { query, response: response.clone() });
    chat.updated_at = Local::now().timestamp();
    chat.save_to_file(&chat_at).handle_error(500)?;

    Ok(Box::new(json(&response)))
}

pub fn create_chat(user: String, repo: String) -> Box<dyn Reply> {
    handler(create_chat_(user, repo))
}

fn create_chat_(user: String, repo: String) -> RawResponse {
    let index_at = join3(
        "data",
        &user,
        &repo,
    ).handle_error(400)?;

    if !exists(&index_at) {
        return Err((404, format!("`{index_at}` not found")));
    }

    let chat_at = join(&index_at, "chats").handle_error(500)?;

    if !exists(&chat_at) {
        create_dir_all(&chat_at).handle_error(500)?;
    }

    let id = format!("{:016x}", rand::random::<u64>());
    let chat = Chat::new(id.clone());
    write_string(
        &join(&chat_at, &set_extension(&id, "json").handle_error(500)?).handle_error(500)?,
        &serde_json::to_string(&chat).handle_error(500)?,
        WriteMode::AlwaysCreate,
    ).handle_error(500)?;

    Ok(Box::new(with_header(
        id,
        "Content-Type",
        "text/plain; charset=utf-8",
    )))
}

fn try_register_session(session_id: u128) -> Result<(), Error> {
    let now = Local::now().timestamp();

    unsafe {
        for i in 0..SESSION_POOL_SIZE {
            match SESSIONS[i] {
                // if there's no update for 10 minutes, it kills the session
                Some(Session { id, last_updated }) if last_updated + 600 < now => {
                    clean_session_fs(id)?;
                    SESSIONS[i] = Some(Session {
                        id: session_id,
                        last_updated: now,
                    });
                    init_session_fs(id)?;

                    return Ok(());
                },
                None => {
                    SESSIONS[i] = Some(Session {
                        id: session_id,
                        last_updated: now,
                    });
                    init_session_fs(session_id)?;

                    return Ok(());
                },
                Some(_) => {},
            }
        }
    }

    Err(Error::ServerBusy)
}

fn try_update_timestamp(session_id: u128) -> Result<(), Error> {
    let now = Local::now().timestamp();

    unsafe {
        for i in 0..SESSION_POOL_SIZE {
            match SESSIONS[i] {
                Some(Session { id, .. }) if id == session_id => {
                    SESSIONS[i] = Some(Session {
                        id: session_id,
                        last_updated: now,
                    });
                    return Ok(());
                },
                _ => {},
            }
        }
    }

    Err(Error::NoSuchSession(session_id))
}

fn try_unregister_session(session_id: u128) -> Result<(), Error> {
    unsafe {
        for i in 0..SESSION_POOL_SIZE {
            match SESSIONS[i] {
                Some(Session { id, .. }) if id == session_id => {
                    SESSIONS[i] = None;
                    clean_session_fs(session_id)?;
                    return Ok(());
                },
                _ => {},
            }
        }
    }

    Err(Error::NoSuchSession(session_id))
}

fn init_session_fs(session_id: u128) -> Result<(), FileError> {
    let path = join(
        "./session",
        &format!("{session_id:032x}"),
    )?;
    create_dir_all(&path)?;
    Ok(())
}

fn clean_session_fs(session_id: u128) -> Result<(), FileError> {
    let path = join(
        "./session",
        &format!("{session_id:032x}"),
    )?;
    remove_dir_all(&path)?;
    Ok(())
}

fn auth(_user: &str, _repo: &str, _auth_info: &Option<(String, Option<String>)>) -> bool {
    // TODO
    true
}
