use crate::utils::trim_long_string;
use ragit_fs::write_log;
use warp::Reply;
use warp::http::status::StatusCode;
use warp::reply::with_status;

mod get;
mod post;

pub use get::{
    get_archive,
    get_archive_list,
    get_cat_file,
    get_chat,
    get_chunk,
    get_chunk_count,
    get_chunk_list,
    get_chunk_list_all,
    get_config,
    get_image,
    get_image_desc,
    get_image_list,
    get_index,
    get_meta,
    get_prompt,
    get_repo_list,
    get_server_version,
    get_user_list,
    get_version,
};
pub use post::{
    create_chat,
    post_archive,
    post_begin_push,
    post_chat,
    post_finalize_push,
};

pub type RawResponse = Result<Box<dyn Reply>, (u16, String)>;

pub fn not_found() -> Box<dyn Reply> {
    Box::new(with_status(String::new(), StatusCode::from_u16(404).unwrap()))
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
