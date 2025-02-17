use warp::Reply;
use warp::http::status::StatusCode;
use warp::reply::with_status;

mod get;
mod post;

pub use get::{
    get_archive,
    get_archive_list,
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
    get_server_version,
    get_version,
};
pub use post::{
    post_archive,
    post_begin_push,
    post_finalize_push,
};

pub fn not_found() -> Box<dyn Reply> {
    Box::new(with_status(String::new(), StatusCode::from_u16(404).unwrap()))
}
