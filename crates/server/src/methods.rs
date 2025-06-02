use crate::error::Error;
use crate::utils::trim_long_string;
use ragit_fs::write_log;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::collections::HashMap;
use std::str::FromStr;
use warp::http::status::StatusCode;
use warp::reply::{Reply, with_header, with_status};

mod admin;
mod ai_model;
mod auth;
mod chat;
mod chunk;
mod clone;
mod health;
mod image;
mod push;

// If an api has to read/write to disk storage (e.g. `index.json`), that's in `repo_fs` module.
// If an api only reads/writes to DB, that's in `repo` module.
mod repo;
mod repo_fs;

mod search;
mod user;

pub use admin::{
    drop_all,
    truncate_all,
};
pub use ai_model::{
    get_ai_model_list,
    put_ai_model_list,
};
pub use auth::{
    create_api_key,
    get_api_key_list,
};
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
pub use push::{
    post_archive,
    post_begin_push,
    post_finalize_push,
};
pub use repo::{
    get_repo,
    get_repo_list,
    get_traffic,
    put_repo,
};
pub use repo_fs::{
    create_repo,
    get_cat_file,
    get_config,
    get_content,
    get_file_content,
    get_index,
    get_meta,
    get_meta_by_key,
    get_prompt,
    get_uid,
    get_version,
    post_build_search_index,
};
pub use search::search;
pub use user::{
    create_user,
    get_user,
    get_user_ai_model_list,
    get_user_list,
    put_user_ai_model_list,
};

static POOL: tokio::sync::OnceCell<PgPool> = tokio::sync::OnceCell::const_new();

async fn get_pool() -> &'static PgPool {
    POOL.get_or_init(|| async {
        write_log(
            "init_pg_pool",
            "start initializing pg pool",
        );

        let database_url = std::env::var("DATABASE_URL").unwrap();

        match PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url).await {
            Ok(pool) => pool,
            Err(e) => {
                write_log(
                    "init_pg_pool",
                    &format!("{e:?}"),
                );
                panic!("{e:?}");
            },
        }
    }).await
}

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

/// It's a boilerplate function for api endpoints. All the functions are supposed to
/// return `Box<dyn Reply>`, but we cannot use the great `?` operator with the type.
///
/// So, ragit-server uses helper functions.
/// Let's say we want to define `get_user(name: &str) -> Box<dyn Reply>`. We first
/// define a helper function `get_user_(name: &str) -> Result<Box<dyn Reply>, (u16, String)>`.
/// Since the return type of the helper function is `Result<_>`, we can use the great `?` operator.
/// Then we have to define a wrapper function `get_user(name: &str) -> Box<dyn Reply>` which
/// uses `handler` to convert the `Result<_>` type to `Box<dyn Reply>`.
pub fn handler(r: RawResponse) -> Box<dyn Reply> {
    match r {
        Ok(r) => r,
        Err((code, error)) => {
            write_log(
                "handler",
                &format!("code: {code}, error: {}", trim_long_string(&error, 200, 200)),
            );

            Box::new(with_status(
                // Let's hide error detail to the clients. I'm not sure whether it's a good idea, tho.
                String::new(),
                StatusCode::from_u16(code).unwrap(),
            ))
        },
    }
}

/// This is a helper trait. It turns a value into a type that's compatible with `handler` function,
/// so that you can use the great `?` operator.
pub trait HandleError<T> {
    fn handle_error(self, code: u16) -> Result<T, (u16, String)>;
}

impl<T, E: std::fmt::Debug> HandleError<T> for Result<T, E> {
    fn handle_error(self, code: u16) -> Result<T, (u16, String)> {
        self.map_err(|e| (code, format!("{e:?}")))
    }
}

impl<T> HandleError<T> for Option<T> {
    fn handle_error(self, code: u16) -> Result<T, (u16, String)> {
        self.ok_or_else(|| (code, format!("expected type `{}`, got `None`", std::any::type_name::<T>())))
    }
}

impl HandleError<()> for bool {
    fn handle_error(self, code: u16) -> Result<(), (u16, String)> {
        if self { Ok(()) } else { Err((code, String::from("expected `true`, got `false`"))) }
    }
}

fn check_secure_path(path: &str) -> Result<(), Error> {
    if path.starts_with(".") || path.contains("/") {
        Err(Error::InsecurePath(path.to_string()))
    }

    else {
        Ok(())
    }
}

pub(crate) fn get_or<T: FromStr>(query: &HashMap<String, String>, key: &str, default_value: T) -> T {
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
