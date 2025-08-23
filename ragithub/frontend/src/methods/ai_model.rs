use super::{
    HandleError,
    RawResponse,
    TERA,
    TeraContextBuilder,
    get_backend,
    get_or,
    handler,
};
use crate::models::AiModel;
use crate::utils::{fetch_json, into_query_string};
use std::collections::HashMap;
use warp::reply::{Reply, html};

pub async fn get_model_index(query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_model_index_(query).await)
}

async fn get_model_index_(query: HashMap<String, String>) -> RawResponse {
    let tera = &TERA;
    let backend = get_backend();
    let mut tera_context = TeraContextBuilder {
        image_modal_box: false,
        markdown: false,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec!["ai-model-index.css"],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();

    let name_query = get_or(&query, "name", String::new());
    let tags_query = get_or(&query, "tags", String::new());

    let limit_query = get_or(&query, "limit", String::new());
    let limit = limit_query.parse::<usize>().unwrap_or(50) + 1;
    let limit_query = limit.to_string();
    let offset_query = get_or(&query, "offset", String::new());
    let offset_query = offset_query.parse::<usize>().unwrap_or(0).to_string();
    let query_str = into_query_string(&[
        ("name", name_query),
        ("tags", tags_query),
        ("limit", limit_query),
        ("offset", offset_query),
    ].into_iter().filter(
        |(_, v)| !v.is_empty()
    ).map(
        |(k, v)| (k.to_string(), v)
    ).collect::<HashMap<_, _>>());

    let mut models = fetch_json::<Vec<AiModel>>(&format!("{backend}/ai-model-list?{query_str}"), &None).await.handle_error(500)?;
    let mut has_more_models = false;

    if models.len() == limit {
        models = models[..(limit - 1)].to_vec();
        has_more_models = true;
    }

    tera_context.insert("models", &models);

    Ok(Box::new(html(tera.render("ai-model-index.html", &tera_context).unwrap())))
}
