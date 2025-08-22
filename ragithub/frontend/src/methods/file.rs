use super::{
    HandleError,
    RawResponse,
    TERA,
    TeraContextBuilder,
    get_backend,
    handler,
};
use crate::models::{ChunkDetail, FileDetail, RenderableChunk};
use crate::utils::{fetch_json, url_encode_strict};
use std::collections::HashMap;
use warp::reply::{Reply, html};

pub async fn get_file(repo: String, query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_file_(repo, query).await)
}

async fn get_file_(repo: String, query: HashMap<String, String>) -> RawResponse {
    let tera = &TERA;
    let backend = get_backend();

    // This page shows exactly what LLM sees.
    // So, it doesn't render markdown.
    // Also, it doesn't use modal images but instead
    // has a dedicated image page.
    let mut tera_context = TeraContextBuilder {
        image_modal_box: false,
        markdown: false,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec!["/content.css"],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();
    let path = match query.get("path") {
        Some(path) => path.to_string(),
        None => {
            return Err((400, String::from("`path` not in the query string")));
        },
    };
    let file = fetch_json::<FileDetail>(&format!("{backend}/sample/{repo}/file-content?path={}", url_encode_strict(&path)), &None).await.handle_error(404)?;
    let chunks = fetch_json::<Vec<ChunkDetail>>(&format!("{backend}/sample/{repo}/search?file={}", url_encode_strict(&path)), &None).await.handle_error(500)?;
    let renderable_chunks = chunks.clone().into_iter().map(|c| c.into()).collect::<Vec<RenderableChunk>>();

    tera_context.insert("repo", &repo);
    tera_context.insert("uid", &file.uid);
    tera_context.insert("content", &file.content);
    tera_context.insert("path", &path);
    tera_context.insert("chunks", &renderable_chunks);

    if chunks.len() == 1 {
        tera_context.insert("title", &chunks[0].title);
        tera_context.insert("summary", &chunks[0].summary);
    }

    Ok(Box::new(html(tera.render("content.html", &tera_context).unwrap())))
}
