use bytes::{Bytes, BufMut};
use chrono::Local;
use crate::utils::{decode_base64, get_rag_path};
use futures_util::TryStreamExt;
use ragit::Index;
use ragit_fs::{
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
    write_bytes,
    write_log,
};
use std::collections::HashMap;
use warp::Reply;
use warp::filters::multipart::FormData;
use warp::http::StatusCode;
use warp::reply::{with_header, with_status};

#[derive(Copy, Clone)]
struct Session {
    id: u128,
    last_updated: i64,
}

// TODO: any better way for global vars
const SESSION_POOL_SIZE: usize = 64;
static mut SESSIONS: [Option<Session>; SESSION_POOL_SIZE] = [None; SESSION_POOL_SIZE];

// TODO: some handlers return 500, some 404 or 503, but I'm not sure which one is correct in which cases

pub fn post_begin_push(user: String, repo: String, auth_info: Option<String>) -> Box<dyn Reply> {
    let session_id = rand::random::<u128>();
    let root_dir = get_rag_path(&user, &repo);
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
        return Box::new(with_status(
            String::new(),
            StatusCode::from_u16(403).unwrap(),
        ));
    }

    if !exists(&root_dir) {
        create_dir_all(&parent(&root_dir).unwrap()).unwrap();
    }

    match try_register_session(session_id) {
        Ok(()) => {
            let session_id = format!("{session_id:032x}");
            Box::new(with_header(
                session_id,
                "Content-Type",
                "text/plain",
            ))
        },
        Err(()) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(503).unwrap(),
        )),
    }
}

pub async fn post_archive(user: String, repo: String, form: FormData) -> Box<dyn Reply> {
    let form: HashMap<String, Vec<u8>> = match form.and_then(|mut field| async move {
        let mut buffer = Vec::new();

        while let Some(content) = field.data().await {
            buffer.put(content?);
        }

        Ok((
            field.name().to_string(),
            buffer,
        ))
    }).try_collect().await {
        Ok(f) => f,
        Err(_) => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(500).unwrap(),
            ));
        },
    };

    let session_id = match form.get("session-id") {
        Some(n) => match u128::from_str_radix(&String::from_utf8_lossy(n), 16) {
            Ok(n) => n,
            Err(_) => {
                return Box::new(with_status(
                    String::new(),
                    StatusCode::from_u16(400).unwrap(),
                ));
            },
        },
        None => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };
    let archive_id = match form.get("archive-id") {
        Some(id) => match String::from_utf8(id.to_vec()) {
            Ok(id) => id,
            Err(_) => {
                return Box::new(with_status(
                    String::new(),
                    StatusCode::from_u16(400).unwrap(),
                ));
            },
        },
        None => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };
    let archive = match form.get("archive") {
        Some(bytes) => bytes,
        None => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };

    if let Err(()) = try_update_timestamp(session_id) {
        return Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        ));
    }

    let Ok(path) = join3(
        "./session",
        &format!("{session_id:032x}"),
        &archive_id,
    ) else {
        return Box::new(with_status(
            String::new(),
            StatusCode::from_u16(500).unwrap(),
        ));
    };

    if let Err(_) = write_bytes(
        &path,
        &archive,
        WriteMode::AlwaysCreate,
    ) {
        return Box::new(with_status(
            String::new(),
            StatusCode::from_u16(500).unwrap(),
        ));
    }

    Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    ))
}

pub fn post_finalize_push(user: String, repo: String, body: Bytes) -> Box<dyn Reply> {
    let session_id = match String::from_utf8(body.into_iter().collect::<Vec<u8>>()) {
        Ok(session_id) => match u128::from_str_radix(&session_id, 16) {
            Ok(n) => n,
            Err(_) => {
                return Box::new(with_status(
                    String::new(),
                    StatusCode::from_u16(400).unwrap(),
                ));
            },
        },
        Err(_) => {
            return Box::new(with_status(
                String::new(),
                StatusCode::from_u16(400).unwrap(),
            ));
        },
    };
    let root_dir = parent(&get_rag_path(&user, &repo)).unwrap();
    let archives_at = join(
        "./session",
        &format!("{session_id:032x}"),
    ).unwrap();
    let archives = read_dir(&archives_at, false).unwrap_or(vec![]);

    if let Err(e) = Index::extract_archive(
        &root_dir,
        archives.clone(),
        4,

        // TODO: is it okay to force-extract? if there's an error, it might lose the original data
        true,
        true,  // quiet
    ) {
        write_log("finalize_push", &format!("{e:?}"));
        return Box::new(with_status(
            String::new(),
            StatusCode::from_u16(500).unwrap(),
        ));
    }

    if !exists(&join3(
        &root_dir,
        ".ragit",
        "archives",
    ).unwrap()) {
        create_dir_all(&join3(
            &root_dir,
            ".ragit",
            "archives",
        ).unwrap()).unwrap();
    }

    for archive in archives.iter() {
        rename(archive, &join4(
            &root_dir,
            ".ragit",
            "archives",
            &file_name(archive).unwrap(),
        ).unwrap()).unwrap();
    }

    match try_unregister_session(session_id) {
        Ok(()) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(200).unwrap(),
        )),
        Err(()) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

fn try_register_session(session_id: u128) -> Result<(), ()> {
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

    // has to wait until the other operations are complete
    Err(())
}

fn try_update_timestamp(session_id: u128) -> Result<(), ()> {
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

    // no such session
    Err(())
}

fn try_unregister_session(session_id: u128) -> Result<(), ()> {
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

    Err(())
}

fn init_session_fs(session_id: u128) -> Result<(), ()> {
    let path = join(
        "./session",
        &format!("{session_id:032x}"),
    ).map_err(|_| ())?;
    create_dir_all(&path).map_err(|_| ())?;
    Ok(())
}

fn clean_session_fs(session_id: u128) -> Result<(), ()> {
    let path = join(
        "./session",
        &format!("{session_id:032x}"),
    ).map_err(|_| ())?;
    remove_dir_all(&path).map_err(|_| ())?;
    Ok(())
}

fn auth(user: &str, repo: &str, auth_info: &Option<(String, Option<String>)>) -> bool {
    // TODO
    true
}
