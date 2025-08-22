use super::{HandleError, RawResponse, TeraContextBuilder, handler};
use crate::TERA;
use crate::models::{BlogIndex, blog};
use ragit_fs::{
    join,
    read_string,
};
use std::collections::HashMap;
use warp::reply::{Reply, html};

pub fn get_blog_index(query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_blog_index_(query))
}

fn get_blog_index_(query: HashMap<String, String>) -> RawResponse {
    let tera = &TERA;
    let mut tera_context = TeraContextBuilder {
        image_modal_box: true,
        markdown: true,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec![],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();

    let j = read_string(&join("blog", "_index.json").handle_error(500)?).handle_error(500)?;
    let mut index = serde_json::from_str::<Vec<BlogIndex>>(&j).handle_error(500)?;

    if let Some(tag) = query.get("tag") {
        index = index.into_iter().filter(
            |index| index.tags.contains(tag)
        ).collect();
        tera_context.insert("tag", tag);
    }

    tera_context.insert("index", &blog::into_renderables(&index));

    Ok(Box::new(html(tera.render("blog-index.html", &tera_context).unwrap())))
}

pub fn get_blog_article(key: String) -> Box<dyn Reply> {
    handler(get_blog_article_(key))
}

fn get_blog_article_(key: String) -> RawResponse {
    let tera = &TERA;
    let mut tera_context = TeraContextBuilder {
        image_modal_box: true,
        markdown: true,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec![],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();

    let j = read_string(&join("blog", "_index.json").handle_error(500)?).handle_error(500)?;
    let index = serde_json::from_str::<Vec<BlogIndex>>(&j).handle_error(500)?;
    let index = match index.iter().filter(|i| i.key == key).next() {
        Some(index) => index.clone(),
        None => {
            return Err((404, format!("`{key}` is an invalid key for a blog article.")));
        },
    };
    let article = read_string(&join("blog", &index.file).handle_error(500)?).handle_error(500)?;

    tera_context.insert("index", &index);
    tera_context.insert("article", &article);

    Ok(Box::new(html(tera.render("blog-article.html", &tera_context).unwrap())))
}
