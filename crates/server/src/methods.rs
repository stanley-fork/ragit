use warp::Reply;
use warp::http::status::StatusCode;
use warp::reply::with_status;

mod get;

pub use get::{
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

pub fn not_found() -> Box<dyn Reply> {
    Box::new(with_status(String::new(), StatusCode::from_u16(404).unwrap()))
}
