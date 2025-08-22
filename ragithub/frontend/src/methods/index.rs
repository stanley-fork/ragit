use super::{
    HandleError,
    RawResponse,
    TeraContextBuilder,
    handler,
    redirect,
};
use crate::TERA;
use crate::models::{RepoIndex, SortCategory, load_repositories};
use crate::utils::into_query_string;
use ragit_fs::read_string;
use serde::Serialize;
use std::collections::HashMap;
use warp::reply::{Reply, html};

pub async fn get_index(query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_index_(query).await)
}

async fn get_index_(query: HashMap<String, String>) -> RawResponse {
    let tera = &TERA;
    let mut tera_context = TeraContextBuilder {
        image_modal_box: true,
        markdown: true,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec!["/sidebar.css"],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();
    let notice = read_string("notice.md").handle_error(500)?;
    let mut sort_by = String::from("t-w");  // default
    let mut sidebar_categories = vec![
        SortCategory {
            long: String::from("trending (weekly)"),
            short: String::from("t-w"),
            selected: true,  // default
            extra_query_string: String::new(),
        },
        SortCategory {
            long: String::from("trending (all)"),
            short: String::from("t-a"),
            selected: false,
            extra_query_string: String::new(),
        },
        SortCategory {
            long: String::from("new"),
            short: String::from("new"),
            selected: false,
            extra_query_string: String::new(),
        },
    ];

    if let Some(s) = query.get("sort_by") {
        for c in sidebar_categories.iter_mut() {
            if c.short == *s {
                c.selected = true;
                sort_by = c.short.to_string();
            }

            else {
                c.selected = false;
            }
        }
    }

    let sidebar_items = {
        let mut repositories = load_repositories().await.handle_error(500)?;

        match sort_by.as_str() {
            "t-w" => {
                repositories.sort_by_key(|r| usize::MAX - r.clone_weekly);
            },
            "t-a" => {
                repositories.sort_by_key(|r| usize::MAX - r.clone_total);
            },
            "new" => {
                repositories.sort_by_key(|r| r.created_at);
                repositories = repositories.into_iter().rev().collect();
            },
            _ => unreachable!(),
        }

        if repositories.len() > 10 {
            repositories = repositories[..10].to_vec();
        }

        SidebarItem::from_repositories(&repositories)
    };

    tera_context.insert("sort_by", &sort_by);
    tera_context.insert("sidebar_categories", &sidebar_categories);
    tera_context.insert("sidebar_items", &sidebar_items);
    tera_context.insert("notice", &notice);

    Ok(Box::new(html(tera.render("index.html", &tera_context).unwrap())))
}

pub fn post_index(form: HashMap<String, String>) -> Box<dyn Reply> {
    handler(post_index_(form))
}

fn post_index_(form: HashMap<String, String>) -> RawResponse {
    Ok(redirect(&format!("/list?{}", into_query_string(&form))))
}

#[derive(Serialize)]
struct SidebarItem {
    rank: usize,
    href: String,
    title: String,
}

impl SidebarItem {
    pub fn from_repositories(repositories: &[RepoIndex]) -> Vec<SidebarItem> {
        repositories.iter().enumerate().map(
            |(i, r)| SidebarItem {
                rank: i + 1,
                title: r.name.clone(),
                href: format!("/sample/{}", r.name),
            }
        ).collect()
    }
}
