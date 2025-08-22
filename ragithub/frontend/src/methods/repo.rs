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
    FILE_VIEWER_LIMIT,
    Repository,
    SortCategory,
    fetch_files,
    load_repositories,
    render_file_entries,
};
use crate::utils::{fetch_json, into_query_string, url_encode_strict};
use std::collections::{HashMap, HashSet};
use warp::reply::{Reply, html};

pub async fn get_repo_index(query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_repo_index_(query).await)
}

async fn get_repo_index_(query: HashMap<String, String>) -> RawResponse {
    let tera = &TERA;
    let mut tera_context = TeraContextBuilder {
        image_modal_box: false,
        markdown: false,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec!["/repo-index.css"],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();

    let mut sort_by = String::from("t-w");  // default
    let search_keyword = get_or(&query, "search", String::new());
    let extra_query_string = if search_keyword.is_empty() { String::new() } else { format!("&search={}", url_encode_strict(&search_keyword)) };
    let mut sort_categories = vec![
        SortCategory {
            long: String::from("trending (weekly)"),
            short: String::from("t-w"),
            selected: true,  // default
            extra_query_string: extra_query_string.clone(),
        },
        SortCategory {
            long: String::from("trending (all)"),
            short: String::from("t-a"),
            selected: false,
            extra_query_string: extra_query_string.clone(),
        },
        SortCategory {
            long: String::from("new"),
            short: String::from("new"),
            selected: false,
            extra_query_string: extra_query_string.clone(),
        },
        SortCategory {
            long: String::from("abc"),
            short: String::from("abc"),
            selected: false,
            extra_query_string: extra_query_string.clone(),
        },
    ];

    if let Some(s) = query.get("sort_by") {
        for c in sort_categories.iter_mut() {
            if c.short == *s {
                c.selected = true;
                sort_by = c.short.clone();
            }

            else {
                c.selected = false;
            }
        }
    }

    let mut repositories = load_repositories().await.handle_error(500)?;

    // this is very very naive search implementation. We might need better implementation when ragithub gets bigger.
    if !search_keyword.is_empty() {
        let s = search_keyword.to_ascii_lowercase();
        repositories = repositories.into_iter().filter(
            |r| {
                let haystack = format!("{} {}", r.name.to_ascii_lowercase(), r.description.as_ref().map(|d| d.clone()).unwrap_or(String::new()).to_ascii_lowercase());
                haystack.contains(&s)
            }
        ).collect();
    }

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
        "abc" => {
            repositories.sort_by_key(|r| r.name.clone());
        },
        _ => unreachable!(),
    }

    tera_context.insert("sort_by", &sort_by);
    tera_context.insert("sort_categories", &sort_categories);
    tera_context.insert("repositories", &repositories);

    if !search_keyword.is_empty() {
        tera_context.insert("search_keyword", &search_keyword);
    }

    Ok(Box::new(html(tera.render("repo-index.html", &tera_context).unwrap())))
}

pub async fn get_repo_detail(repo: String, query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_repo_detail_(repo, query).await)
}

async fn get_repo_detail_(repo: String, query: HashMap<String, String>) -> RawResponse {
    let tera = &TERA;
    let backend = get_backend();
    let repository = fetch_json::<Repository>(&format!("{backend}/repo-list/sample/{repo}"), &None).await.handle_error(500)?;
    let readme = fetch_json::<Option<String>>(&format!("{backend}/sample/{repo}/meta/readme"), &None).await.handle_error(500)?;
    let (clone_weekly, clone_total) = {
        let repositories = load_repositories().await.handle_error(500)?;
        let mut clone_weekly = 0;
        let mut clone_total = 0;

        for r in repositories.iter() {
            if r.name == repo {
                clone_weekly = r.clone_weekly;
                clone_total = r.clone_total;
                break;
            }
        }

        (clone_weekly, clone_total)
    };
    let expand = match query.get("expand") {
        Some(expand) => expand.as_bytes().chunks(7).map(
            |chunk| String::from_utf8_lossy(chunk).to_string()
        ).collect(),
        None => HashSet::new(),
    };
    let mut exceeded_limit = false;
    let files = fetch_files("/", &repo, &expand, &mut exceeded_limit).await.handle_error(500)?;
    let mut tera_context = TeraContextBuilder {
        image_modal_box: true,
        markdown: true,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec!["/repo-detail.css"],
        extra_scripts: vec!["/clone-button.js"],
        extra_components: vec![],
    }.build();

    tera_context.insert("description", &repository.description);
    tera_context.insert("website", &repository.website);
    tera_context.insert("name", &repository.name);
    tera_context.insert("file_entries", &render_file_entries(&repo, &files, vec![], &expand));
    tera_context.insert("created_at", &repository.created_at);
    tera_context.insert("updated_at", &repository.updated_at);
    tera_context.insert("clone_weekly", &clone_weekly);
    tera_context.insert("clone_total", &clone_total);
    tera_context.insert("readme", &readme.as_ref().map(|s| s.as_str()).unwrap_or(""));

    if exceeded_limit {
        tera_context.insert("popup", &format!("Some directories have more than {FILE_VIEWER_LIMIT} files. We had to truncate them to {FILE_VIEWER_LIMIT} files."));
    }

    Ok(Box::new(html(tera.render("repo-detail.html", &tera_context).unwrap())))
}

pub fn search_repo(form: HashMap<String, String>) -> Box<dyn Reply> {
    handler(search_repo_(form))
}

fn search_repo_(form: HashMap<String, String>) -> RawResponse {
    Ok(redirect(&format!("/list?{}", into_query_string(&form))))
}
