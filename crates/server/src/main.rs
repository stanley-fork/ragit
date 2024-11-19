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
- GET `ROOT/{user-name}/{repo-name}/chunk-list`
  - 200: application/json
  - 404
- GET `ROOT/{user-name}/{repo-name}/chunk/{chunk-uid}`
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
- GET `ROOT/{user-name}/{repo-name}/version`
  - 200: text/plain
  - 404
- GET `ROOT/version`
  - 200: text/plain
*/

use crate::methods::*;
use ragit_fs::{
    initialize_log_file,
    set_log_file_path,
    write_log,
};
use warp::Filter;

mod methods;
mod utils;

#[tokio::main]
async fn main() {
    set_log_file_path(Some("ragit-server-logs".to_string()));
    initialize_log_file("ragit-server-logs", true).unwrap();

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

    let get_chunk_list_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("chunk-list"))
        .map(get_chunk_list);

    let get_chunk_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("chunk"))
        .and(warp::path::param::<String>())
        .map(get_chunk);

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

    let get_version_handler = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path("version"))
        .map(get_version);

    let get_server_version_handler = warp::get()
        .and(warp::path("version"))
        .map(get_server_version);

    let not_found_handler = warp::get().map(not_found);

    warp::serve(
        get_server_version_handler
            .or(get_index_handler)
            .or(get_config_handler)
            .or(get_prompt_handler)
            .or(get_chunk_list_handler)
            .or(get_chunk_handler)
            .or(get_image_list_handler)
            .or(get_image_handler)
            .or(get_image_desc_handler)
            .or(get_meta_handler)
            .or(get_version_handler)
            .or(not_found_handler)
            .with(warp::log::custom(
                |info| {
                    let headers = info.request_headers();

                    write_log(
                        &info.remote_addr().map(
                            |remote_addr| remote_addr.to_string()
                        ).unwrap_or_else(|| String::from("NO REMOTE ADDR")),
                        &format!(
                            "{:4} {:16} {:4} {headers:?}",
                            info.method().as_str(),
                            info.path(),
                            info.status().as_u16(),
                        ),
                    );
                }
            ))
    ).run(([0, 0, 0, 0], 41127)).await;  // TODO: configurable port number
}
