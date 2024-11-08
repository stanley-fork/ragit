/*
- GET `ROOT/{user-name}/{repo-name}/index`
  - returns index.json
  - 200: application/json
  - 404
- GET `ROOT/{user-name}/{repo-name}/config/{config-name}`
  - 200: application/json
  - 404
- GET `ROOT/{user-name}/{repo-name}/prompt/{prompt-name}`
  - 200: text/plain
  - 404
- GET `ROOT/{user-name}/{repo-name}/chunk-file-list`
  - 200: application/json
  - 404
- GET `ROOT/{user-name}/{repo-name}/chunk-file/{chunk-file-name}`
  - 200: application/octet-stream
  - 404
- GET `ROOT/{user-name}/{repo-name}/image-list`
  - 200: application/json
  - 404
- GET `ROOT/{user-name}/{repo-name}/image/{image-name}`
  - 200: image/png
  - 404
- GET `ROOT/{user-name}/{repo-name}/image-desc/{image-name}`
  - 200: application/json
  - 404
- GET `ROOT/{user-name}/{repo-name}/meta`
  - 200: application/json
  - 404
*/

use crate::methods::*;
use warp::Filter;

mod methods;
mod utils;

#[tokio::main]
async fn main() {
    let get_index_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("index"))
        .map(get_index);

    let get_config_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("config"))
        .and(warp::path::param::<String>())
        .map(get_config);

    let get_prompt_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("prompt"))
        .and(warp::path::param::<String>())
        .map(get_prompt);

    let get_chunk_file_list_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("chunk-file-list"))
        .map(get_chunk_file_list);

    let get_chunk_file_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("chunk-file"))
        .and(warp::path::param::<String>())
        .map(get_chunk_file);

    let get_image_list_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("image-list"))
        .map(get_image_list);

    let get_image_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("image"))
        .and(warp::path::param::<String>())
        .map(get_image);

    let get_image_desc_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("image-desc"))
        .and(warp::path::param::<String>())
        .map(get_image_desc);

    let get_meta_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("meta"))
        .map(get_meta);

    let not_found_handler = warp::get().map(not_found);

    warp::serve(
        get_index_handler
            .or(get_config_handler)
            .or(get_prompt_handler)
            .or(get_chunk_file_list_handler)
            .or(get_chunk_file_handler)
            .or(get_image_list_handler)
            .or(get_image_handler)
            .or(get_image_desc_handler)
            .or(get_meta_handler)
            .or(not_found_handler)
    ).run(([0, 0, 0, 0], 41127)).await;  // TODO: configurable port number
}
