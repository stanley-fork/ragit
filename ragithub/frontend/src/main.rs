#![recursion_limit = "256"]

use bytes::Bytes;
use crate::error::Error;
use crate::init::init_server;
use crate::methods::*;
use ragit_cli::{
    ArgParser,
    ArgType,
};
use ragit_fs::{
    basename,
    join,
    read_dir,
    set_current_dir,
    write_log,
};
use ragit_pdl::ImageType;
use ragit_server::utils::fetch_form_data;
use std::collections::HashMap;
use std::env;
use warp::Filter;
use warp::filters::multipart::FormData;
use warp::http::StatusCode;
use warp::reply::with_status;

mod colors;
mod error;
mod init;
mod methods;
mod models;
mod utils;

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<_>>();
    let parsed_args = ArgParser::new()
        .optional_arg_flag("--port", ArgType::integer_between(Some(0), Some(65535)))
        .arg_flag_with_default("--backend", "http://127.0.0.1:41127", ArgType::String)
        .short_flag(&["--port"])
        .parse(&args, 1);

    let parsed_args = match parsed_args {
        Ok(parsed_args) => parsed_args,
        Err(e) => {
            let message = e.kind.render();
            eprintln!(
                "cli error: {message}{}",
                if let Some(span) = &e.span {
                    format!("\n\n{}", ragit_cli::underline_span(span))
                } else {
                    String::new()
                },
            );

            std::process::exit(1)
        },
    };
    let port_number = parsed_args.arg_flags.get("--port").map(|n| n.parse::<u16>().unwrap()).unwrap_or(8080);
    set_backend(parsed_args.arg_flags.get("--backend").unwrap());

    goto_root().unwrap();
    init_server().unwrap();
    tokio::spawn(methods::background_worker());

    // GET `/`
    let get_index_handler = warp::get()
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::path::end())
        .then(get_index)
        .or(warp::fs::dir("./static"));

    // POST `/`
    let post_index_handler = warp::post()
        .and(warp::path::end())
        .and(warp::body::form::<HashMap<String, String>>())
        .map(post_index);

    // GET `/list`
    let get_repo_index_handler = warp::get()
        .and(warp::path("list"))
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::path::end())
        .then(get_repo_index)
        .or(warp::fs::dir("./static"));

    // POST `/list`
    let search_repo_handler = warp::post()
        .and(warp::path("list"))
        .and(warp::path::end())
        .and(warp::body::form::<HashMap<String, String>>())
        .map(search_repo);

    // GET `/sample/<repo>`
    let get_repo_detail_handler = warp::get()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::path::end())
        .then(get_repo_detail)
        .or(warp::fs::dir("./static"));

    // GET `/sample/<repo>/file?path=`
    let get_file_handler = warp::get()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("file"))
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::path::end())
        .then(get_file)
        .or(warp::fs::dir("./static"));

    // GET `/sample/<repo>/chunk/<uid>`
    let get_chunk_handler = warp::get()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("chunk"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .then(get_chunk)
        .or(warp::fs::dir("./static"));

    // It's used to provide image files that do not belong to any knowledge-base,
    // but belong directly to ragithub. (e.g. images in notice.md)
    // GET `/i/<image-file>`
    let image_fetch = warp::get()
        .and(warp::path("i"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .map(
            |id: String| {
                if id.starts_with(".") || id.contains("/") {
                    Box::new(warp::reply::with_status(
                        String::new(),
                        warp::http::StatusCode::from_u16(400).unwrap(),
                    )) as Box<dyn warp::Reply>
                }

                else {
                    let image_type = ImageType::infer_from_path(&id).unwrap_or(ImageType::Png);
                    let file_name = if id.to_ascii_lowercase().ends_with(&format!(".{}", image_type.to_extension())) {
                        id.clone()
                    } else {
                        format!("{id}.{}", image_type.to_extension())
                    };

                    if let Ok(path_to_image) = ragit_fs::join("./images", &file_name) {
                        if let Ok(bytes) = ragit_fs::read_bytes(&path_to_image) {
                            Box::new(warp::reply::with_header(
                                bytes,
                                "Content-Type",
                                image_type.get_media_type(),
                            )) as Box<dyn warp::Reply>
                        }

                        else {
                            Box::new(warp::reply::with_status(
                                String::new(),
                                warp::http::StatusCode::from_u16(404).unwrap(),
                            ))
                        }
                    }

                    else {
                        Box::new(warp::reply::with_status(
                            String::new(),
                            warp::http::StatusCode::from_u16(404).unwrap(),
                        ))
                    }
                }
            }
        );

    // GET `/sample/<repo>/i/<uid>`
    let fetch_repo_image_handler = warp::get()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("i"))
        .and(warp::path::param::<String>())
        .then(fetch_repo_image);

    // GET `/sample/<repo>/img/<uid>`
    let get_image_detail_handler = warp::get()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("img"))
        .and(warp::path::param::<String>())
        .then(get_image_detail)
        .or(warp::fs::dir("./static"));

    // GET `/ci`
    let get_ci_index_handler = warp::get()
        .and(warp::path("ci"))
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::path::end())
        .map(get_ci_index)
        .or(warp::fs::dir("./static"));

    // GET `/ci/<title>`
    let get_ci_detail_handler = warp::get()
        .and(warp::path("ci"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .map(get_ci_detail)
        .or(warp::fs::dir("./static"));

    // GET `/ci/<test>/history`
    let get_ci_history_handler = warp::get()
        .and(warp::path("ci"))
        .and(warp::path::param::<String>())
        .and(warp::path("history"))
        .map(get_ci_history)
        .or(warp::fs::dir("./static"));

    // GET `/ci/<title>/download-json`
    let download_json_handler = warp::get()
        .and(warp::path("ci"))
        .and(warp::path::param::<String>())
        .and(warp::path("download-json"))
        .and(warp::path::end())
        .map(download_json);

    // GET `/blog`
    let get_blog_index_handler = warp::get()
        .and(warp::path("blog"))
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::path::end())
        .map(get_blog_index)
        .or(warp::fs::dir("./static"));

    // GET `/blog/<article-id>`
    let get_blog_article_handler = warp::get()
        .and(warp::path("blog"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .map(get_blog_article)
        .or(warp::fs::dir("./static"));

    // GET `/health`
    let get_health_handler = warp::get()
        .and(warp::path("health"))
        .and(warp::path::end())
        .then(get_health);

    // Some execuses for these weird-looking proxies
    // Ragit is designed to speak directly to ragit-server when
    // cloning or pushing. It doesn't know anything about ragithub.
    // But I don't want to expose ragit-server endpoints on the web.
    // The best way would be to use a load balancer or a proxy
    // server, but I don't know how to do that. So I just implement
    // the proxies in ragithub.

    // GET `/sample/<repo>/archive/<archive-id>`
    let clone_proxy_1 = warp::get()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("archive"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::header::optional::<String>("x-api-key"))
        .then(
            |repo: String, archive_id: String, api_key: Option<String>| async move {
                ProxyBuilder {
                    method: Method::Get,
                    api_key,
                    path: vec![
                        String::from("sample"),
                        repo,
                        String::from("archive"),
                        archive_id,
                    ],
                    response_type: ResponseType::Bytes,
                    ..ProxyBuilder::default()
                }.send().await
            }
        );

    // GET `/sample/<repo>/archive-list`
    let clone_proxy_2 = warp::get()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("archive-list"))
        .and(warp::path::end())
        .and(warp::header::optional::<String>("x-api-key"))
        .then(
            |repo: String, api_key: Option<String>| async move {
                ProxyBuilder {
                    method: Method::Get,
                    api_key,
                    path: vec![
                        String::from("sample"),
                        repo,
                        String::from("archive-list"),
                    ],
                    response_type: ResponseType::Json,
                    ..ProxyBuilder::default()
                }.send().await
            }
        );

    // GET `/sample/<repo>/uid`
    let push_proxy_1 = warp::get()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("uid"))
        .and(warp::path::end())
        .and(warp::header::optional::<String>("x-api-key"))
        .then(
            |repo: String, api_key: Option<String>| async move {
                ProxyBuilder {
                    method: Method::Get,
                    api_key,
                    path: vec![
                        String::from("sample"),
                        repo,
                        String::from("uid"),
                    ],
                    response_type: ResponseType::String,
                    ..ProxyBuilder::default()
                }.send().await
            }
        );

    // POST `/sample/<repo>/begin-push`
    let push_proxy_2 = warp::post()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("begin-push"))
        .and(warp::path::end())
        .and(warp::header::optional::<String>("x-api-key"))
        .then(
            |repo: String, api_key: Option<String>| async move {
                ProxyBuilder {
                    method: Method::Post,
                    api_key,
                    path: vec![
                        String::from("sample"),
                        repo,
                        String::from("begin-push"),
                    ],
                    response_type: ResponseType::String,
                    ..ProxyBuilder::default()
                }.send().await
            }
        );

    // POST `/sample/<repo>/archive` (multipart)
    let push_proxy_3 = warp::post()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("archive"))
        .and(warp::path::end())
        .and(warp::multipart::form())
        .and(warp::header::optional::<String>("x-api-key"))
        .then(
            |repo: String, form: FormData, api_key: Option<String>| async move {
                match fetch_form_data(form).await {
                    Ok(form) => ProxyBuilder {
                        method: Method::Post,
                        api_key,
                        path: vec![
                            String::from("sample"),
                            repo,
                            String::from("archive"),
                        ],
                        response_type: ResponseType::None,
                        body_multiparts: Some(form),
                        ..ProxyBuilder::default()
                    }.send().await,
                    Err(e) => {
                        write_log(
                            "fetch_form_data",
                            &format!("{e:?}"),
                        );
                        Box::new(with_status(
                            String::new(),
                            StatusCode::from_u16(500).unwrap(),
                        ))
                    },
                }
            }
        );

    // POST `/sample/<repo>/finalize-push` (body::bytes)
    let push_proxy_4 = warp::post()
        .and(warp::path("sample"))
        .and(warp::path::param::<String>())
        .and(warp::path("finalize-push"))
        .and(warp::path::end())
        .and(warp::body::bytes())
        .and(warp::header::optional::<String>("x-api-key"))
        .then(
            |repo: String, body: Bytes, api_key: Option<String>| async move {
                ProxyBuilder {
                    method: Method::Post,
                    api_key,
                    path: vec![
                        String::from("sample"),
                        repo,
                        String::from("finalize-push"),
                    ],
                    response_type: ResponseType::None,
                    body_raw: Some(body.into_iter().collect()),
                    ..ProxyBuilder::default()
                }.send().await
            }
        );

    let model_proxy_1 = warp::get()
        .and(warp::path("ai-model-list"))
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::path::end())
        .then(
            |query: HashMap<String, String>| async move {
                ProxyBuilder {
                    method: Method::Get,
                    api_key: None,
                    path: vec![String::from("ai-model-list")],
                    response_type: ResponseType::Json,
                    query,
                    ..ProxyBuilder::default()
                }.send().await
            }
        );

    let not_found_handler = warp::get().map(
        || Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        ))
    );

    warp::serve(
        get_index_handler
            .or(post_index_handler)
            .or(get_repo_index_handler)
            .or(get_repo_detail_handler)
            .or(search_repo_handler)
            .or(get_file_handler)
            .or(get_chunk_handler)
            .or(image_fetch)
            .or(fetch_repo_image_handler)
            .or(get_image_detail_handler)
            .or(get_ci_index_handler)
            .or(get_ci_detail_handler)
            .or(get_ci_history_handler)
            .or(download_json_handler)
            .or(get_blog_index_handler)
            .or(get_blog_article_handler)
            .or(get_health_handler)
            .or(clone_proxy_1)
            .or(clone_proxy_2)
            .or(push_proxy_1)
            .or(push_proxy_2)
            .or(push_proxy_3)
            .or(push_proxy_4)
            .or(model_proxy_1)
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
    ).run(([0, 0, 0, 0], port_number)).await;
}

fn goto_root() -> Result<(), Error> {
    let mut curr = String::from(".");

    loop {
        let curr_files = read_dir(&curr, false)?;

        for f in curr_files.iter() {
            if basename(f)? == "Cargo.toml" {
                set_current_dir(&curr)?;
                return Ok(());
            }
        }

        curr = join(&curr, "..")?;
    }
}
