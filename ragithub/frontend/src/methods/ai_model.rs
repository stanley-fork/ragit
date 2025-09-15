use super::{
    HandleError,
    RawResponse,
    TERA,
    TeraContextBuilder,
    get_backend,
    get_or,
    handler,
    redirect,
};
use crate::models::{
    AiModel,
    Tag,
    load_tags,
};
use crate::utils::{
    fetch_json,
    into_query_string,
    url_encode_strict,
};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::hash_map::{Entry, HashMap};
use warp::reply::{Reply, html};

const HELP_MESSAGE: &str = "
Ragit uses AI models to answer your queries. You can use various AI models.
Most models are pre-installed, so you don't have to download one from here.

### Changing models

You can use `rag config --set model gpt-4o` command to change the model. If
the model is not pre-installed, it will throw an error.

### Downloading models

Ragit's model list can be found at `.ragit/models.json` (local) or
`~/.config/ragit` (global, if exists). You can either edit the json file
manually, or use `rag model` command.

Using `rag model` command is simple. You'll see download button in each model
card. Click the download button, paste the command, and run it. Then you'll
have the model in your knowledge-base.

### List models

You can use `rag ls-models` command to see available models in your local
machine.
";

pub async fn get_ai_model_index(query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_ai_model_index_(query).await)
}

async fn get_ai_model_index_(query: HashMap<String, String>) -> RawResponse {
    let tera = &TERA;
    let backend = get_backend();
    let mut tera_context = TeraContextBuilder {
        image_modal_box: false,
        markdown: true,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        tooltip: true,
        show_version: true,
        include_svgs: true,
        extra_styles: vec!["ai-model-index.css"],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();

    let name_query = get_or(&query, "name", String::new());
    let tags_query = get_or(&query, "tags", String::new());

    let limit_query = get_or(&query, "limit", String::new());
    let mut limit = limit_query.parse::<usize>().unwrap_or(50) + 1;
    let limit_query = limit.to_string();
    let offset_query = get_or(&query, "offset", String::new());
    let offset = offset_query.parse::<usize>().unwrap_or(0);
    let offset_query = offset.to_string();
    let api_query_str = into_query_string(&[
        ("name", name_query.clone()),
        ("tags", tags_query.clone()),
        ("limit", limit_query),
        ("offset", offset_query),
    ].into_iter().filter(
        |(_, v)| !v.is_empty()
    ).map(
        |(k, v)| (k.to_string(), v)
    ).collect::<HashMap<_, _>>());

    let mut models = fetch_json::<Vec<AiModel>>(&format!("{backend}/ai-model-list?{api_query_str}"), &None).await.handle_error(500)?;
    let mut has_more_models = false;

    if models.len() == limit {
        models = models[..(limit - 1)].to_vec();
        has_more_models = true;
    }

    limit -= 1;
    let query_str = api_query_str.replace(
        &format!("limit={}", limit + 1),
        &format!("limit={}", limit),
    );

    for model in models.iter_mut() {
        model.tags.sort_by_key(|tag| tag_sort_key(tag));
    }

    let tags = load_tags().await.handle_error(500)?;
    let tags_href_base = format!(
        "/ai-model?name={}&tags={}",
        url_encode_strict(&name_query),
        url_encode_strict(&tags_query),
    );
    let mut tags: HashMap<String, Tag> = tags.into_iter().map(
        |name| (
            name.to_string(),
            Tag {
                name: name.to_string(),
                selected: false,
                href: format!("{tags_href_base}%2C{name}"),
            },
        )
    ).collect();

    for tag_str in tags_query.split(",") {
        if tag_str.is_empty() {
            continue;
        }

        let new_tags_query = tags_query.split(",").filter(
            |tag| tag != &tag_str && !tag.is_empty()
        ).collect::<Vec<_>>().join(",");
        let new_tags_href = format!(
            "/ai-model?name={}&tags={}",
            url_encode_strict(&name_query),
            url_encode_strict(&new_tags_query),
        );
        match tags.entry(tag_str.to_string()) {
            Entry::Occupied(mut tag) => {
                let tag = tag.get_mut();
                tag.selected = true;
                tag.href = new_tags_href.clone();
            },
            Entry::Vacant(e) => {
                e.insert(Tag {
                    name: tag_str.to_string(),
                    selected: true,
                    href: new_tags_href.clone(),
                });
            },
        }
    }

    if !name_query.is_empty() {
        tera_context.insert("name_query", &name_query);

        for model in models.iter_mut() {
            model.name = model.name.replace(&name_query, &format!("<span class=\"model-name-highlight\">{name_query}</span>"));
        }
    }

    let mut tags: Vec<Tag> = tags.into_values().collect();
    tags.sort_by_key(|tag| tag_sort_key(&tag.name));

    tera_context.insert("tags_str", &tags_query);
    tera_context.insert("tags", &tags);
    tera_context.insert("models", &models);

    if offset > 0 {
        let new_offset = offset.max(limit) - limit;
        let new_query_str = query_str.replace(
            &format!("offset={offset}"),
            &format!("offset={new_offset}"),
        );
        tera_context.insert(
            "prev_page_href",
            &format!("/ai-model?{new_query_str}"),
        );
    }

    if has_more_models {
        let new_offset = offset + limit;
        let new_query_str = query_str.replace(
            &format!("offset={offset}"),
            &format!("offset={new_offset}"),
        );
        tera_context.insert(
            "next_page_href",
            &format!("/ai-model?{new_query_str}"),
        );
    }

    tera_context.insert("model_seq_start", &(offset + 1));
    tera_context.insert("model_seq_end", &(offset + models.len()));
    tera_context.insert("help_message", HELP_MESSAGE);
    Ok(Box::new(html(tera.render("ai-model-index.html", &tera_context).unwrap())))
}

pub fn post_ai_model_index(form: HashMap<String, String>) -> Box<dyn Reply> {
    handler(post_ai_model_index_(form))
}

fn post_ai_model_index_(form: HashMap<String, String>) -> RawResponse {
    Ok(Box::new(redirect(&format!("/ai-model?{}", into_query_string(&form)))))
}

lazy_static! {
    static ref PARAM_COUNT_RE: Regex = Regex::new(r"(\d+)(m|M|b|B|t|T)").unwrap();
}

// 1. param-count tags come after the other tags.
// 2. param-count tags are sorted by the actual count.
// 3. Otherwise, sort in abc order
fn tag_sort_key(tag: &str) -> String {
    match PARAM_COUNT_RE.captures(tag) {
        Some(cap) => {
            let n = cap.get(1).unwrap().as_str().parse::<u64>().unwrap();
            let multiplier = match cap.get(2).unwrap().as_str() {
                "m" | "M" => 1_000_000,
                "b" | "B" => 1_000_000_000,
                "t" | "T" => 1_000_000_000_000,
                _ => unreachable!(),
            };

            let count = format!("{:016x}", n * multiplier);
            format!("z-{count}")
        },
        None => format!("a-{tag}"),
    }
}
