use super::{
    HandleError,
    RawResponse,
    TERA,
    TeraContextBuilder,
    get_backend,
    handler,
};
use crate::models::ImageDescription;
use crate::utils::{fetch_bytes, fetch_json};
use warp::reply::{Reply, html, with_header};

pub async fn fetch_repo_image(repo: String, uid: String) -> Box<dyn Reply> {
    handler(fetch_repo_image_(repo, uid).await)
}

async fn fetch_repo_image_(repo: String, uid: String) -> RawResponse {
    let backend = get_backend();
    let bytes = fetch_bytes(&format!("{backend}/sample/{repo}/image/{uid}"), &None).await.handle_error(404)?;

    Ok(Box::new(with_header(
        bytes,
        "Content-Type",
        "image/png",
    )))
}

pub async fn get_image_detail(repo: String, uid: String) -> Box<dyn Reply> {
    handler(get_image_detail_(repo, uid).await)
}

async fn get_image_detail_(repo: String, uid: String) -> RawResponse {
    let tera = &TERA;
    let backend = get_backend();
    let mut tera_context = TeraContextBuilder {
        image_modal_box: true,
        markdown: false,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec!["/content.css"],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();
    let info = fetch_json::<ImageDescription>(&format!("{backend}/sample/{repo}/image-desc/{uid}"), &None).await.handle_error(500)?;

    tera_context.insert("repo", &repo);
    tera_context.insert("uid", &uid);
    tera_context.insert("extracted_text", &info.extracted_text);
    tera_context.insert("explanation", &info.explanation);

    Ok(Box::new(html(tera.render("image-content.html", &tera_context).unwrap())))
}
