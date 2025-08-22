use super::{
    HandleError,
    RawResponse,
    TERA,
    TeraContextBuilder,
    get_backend,
    handler,
};
use crate::models::{ChunkDetail, RenderableChunk};
use crate::utils::fetch_json;
use warp::reply::{Reply, html};

pub async fn get_chunk(repo: String, uid: String) -> Box<dyn Reply> {
    handler(get_chunk_(repo, uid).await)
}

async fn get_chunk_(repo: String, uid: String) -> RawResponse {
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
    let chunk = fetch_json::<ChunkDetail>(&format!("{backend}/sample/{repo}/chunk/{uid}"), &None).await.handle_error(500)?;
    let renderable_chunk = RenderableChunk::from(chunk.clone());

    tera_context.insert("repo", &repo);
    tera_context.insert("path", &renderable_chunk.source);
    tera_context.insert("uid", &chunk.uid);
    tera_context.insert("content", &chunk.data);
    tera_context.insert("title", &chunk.title);
    tera_context.insert("summary", &chunk.summary);

    Ok(Box::new(html(tera.render("content.html", &tera_context).unwrap())))
}