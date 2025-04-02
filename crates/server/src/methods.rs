use crate::utils::trim_long_string;
use ragit_fs::{
    file_name,
    is_dir,
    join,
    read_dir,
    write_log,
};
use warp::http::status::StatusCode;
use warp::reply::{Reply, json, with_header, with_status};

mod chat;
mod chunk;
mod clone;
mod health;
mod image;
mod index;
mod push;
mod search;

pub use chat::{
    create_chat,
    get_chat,
    get_chat_list,
    post_chat,
};
pub use chunk::{
    get_chunk,
    get_chunk_count,
    get_chunk_list,
    get_chunk_list_all,
};
pub use clone::{
    get_archive,
    get_archive_list,
};
pub use health::{
    get_health,
};
pub use image::{
    get_image,
    get_image_desc,
    get_image_list,
};
pub use index::{
    get_cat_file,
    get_config,
    get_file_list,
    get_index,
    get_meta,
    get_prompt,
    get_version,
    post_ii_build,
};
pub use push::{
    post_archive,
    post_begin_push,
    post_finalize_push,
};
pub use search::search;

pub type RawResponse = Result<Box<dyn Reply>, (u16, String)>;

pub fn not_found() -> Box<dyn Reply> {
    Box::new(with_status(String::new(), StatusCode::from_u16(404).unwrap()))
}

pub fn get_server_version() -> Box<dyn Reply> {
    Box::new(with_header(
        ragit::VERSION,
        "Content-Type",
        "text/plain; charset=utf-8",
    ))
}

pub fn get_user_list() -> Box<dyn Reply> {
    handler(get_user_list_())
}

fn get_user_list_() -> RawResponse {
    let dir = read_dir("data", true).unwrap_or(vec![]);
    let mut users = vec![];

    for d in dir.iter() {
        if is_dir(d) {
            users.push(file_name(d).handle_error(500)?);
        }
    }

    Ok(Box::new(json(&users)))
}

pub fn get_repo_list(user: String) -> Box<dyn Reply> {
    handler(get_repo_list_(user))
}

fn get_repo_list_(user: String) -> RawResponse {
    let dir = read_dir(&join("data", &user).handle_error(400)?, true).handle_error(404)?;
    let mut repos = vec![];

    for d in dir.iter() {
        if is_dir(d) {
            repos.push(file_name(d).handle_error(500)?);
        }
    }

    Ok(Box::new(json(&repos)))
}

pub fn handler(r: RawResponse) -> Box<dyn Reply> {
    match r {
        Ok(r) => r,
        Err((code, error)) => Box::new(with_status(
            error,
            StatusCode::from_u16(code).unwrap(),
        )),
    }
}

pub trait HandleError<T> {
    fn handle_error(self, code: u16) -> Result<T, (u16, String)>;
}

impl<T, E: std::fmt::Debug> HandleError<T> for Result<T, E> {
    fn handle_error(self, code: u16) -> Result<T, (u16, String)> {
        self.map_err(|e| {
            let e = format!("{e:?}");
            write_log(
                "handle_error",
                &format!("{code}, {}", trim_long_string(&e, 200, 200)),
            );

            // let's not expose the error message to the client
            // (code, e)
            (code, String::new())
        })
    }
}

fn auth(_user: &str, _repo: &str, _auth_info: &Option<(String, Option<String>)>) -> bool {
    // TODO
    true
}
