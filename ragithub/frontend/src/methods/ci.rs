use super::{HandleError, RawResponse, TeraContextBuilder, handler};
use crate::TERA;
use crate::models::{
    CiDetail,
    CiHistoryDetail,
    CiIndex,
    SortCategory,
    ci,
};
use ragit_fs::{
    join,
    join3,
    read_string,
    write_log,
};
use std::collections::HashMap;
use warp::reply::{Reply, html, with_header};

pub fn get_ci_index(query: HashMap<String, String>) -> Box<dyn Reply> {
    handler(get_ci_index_(query))
}

fn get_ci_index_(query: HashMap<String, String>) -> RawResponse {
    let tera = &TERA;
    let mut tera_context = TeraContextBuilder {
        image_modal_box: false,
        markdown: true,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec!["/ci-page.css"],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();

    let mut sort_by = String::from("desc");  // default
    let mut sort_categories = vec![
        SortCategory {
            long: String::from("Newest to oldest"),
            short: String::from("desc"),
            selected: true,  // default
            extra_query_string: String::new(),
        },
        SortCategory {
            long: String::from("Oldest to newest"),
            short: String::from("asc"),
            selected: false,
            extra_query_string: String::new(),
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

    let j = read_string(&join("test-results", "_index.json").handle_error(500)?).handle_error(500)?;

    // by default, it's sorted in descending order
    let mut index = serde_json::from_str::<Vec<CiIndex>>(&j).handle_error(500)?;

    if sort_by == "asc" {
        index = index.into_iter().rev().collect();
    }

    tera_context.insert("index", &index);
    tera_context.insert("sort_categories", &sort_categories);

    Ok(Box::new(html(tera.render("ci-index.html", &tera_context).unwrap())))
}

pub fn get_ci_detail(title: String) -> Box<dyn Reply> {
    handler(get_ci_detail_(title))
}

fn get_ci_detail_(title: String) -> RawResponse {
    let tera = &TERA;
    let mut tera_context = TeraContextBuilder {
        image_modal_box: false,
        markdown: true,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec!["/ci-page.css"],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();

    if title.starts_with(".") || title.contains("/") {
        return Err((400, format!("malicious input for `get_ci_detail`: {title:?}")));
    }

    let detail = read_string(
        &join(
            "test-results",
            &format!("result-{title}.json"),
        ).handle_error(400)?,
    ).handle_error(404)?;
    let detail = serde_json::from_str::<CiDetail>(&detail).handle_error(500)?;
    let prev_detail = ci::get_prev_detail(&title).unwrap_or_else(
        |e| {
            write_log(
                "get_prev_detail",
                &format!("{e:?}"),
            );
            None
        }
    );
    let next_detail = ci::get_next_detail(&title).unwrap_or_else(
        |e| {
            write_log(
                "get_next_detail",
                &format!("{e:?}"),
            );
            None
        }
    );

    let cases = ci::into_renderables(&detail.tests, false).handle_error(500)?;

    tera_context.insert("title", &title);
    tera_context.insert("detail", &detail);
    tera_context.insert("cases", &cases);

    tera_context.insert("show_description", &false);
    tera_context.insert("show_download_button", &true);
    tera_context.insert("show_git_info", &true);
    tera_context.insert("show_metadata", &true);
    tera_context.insert("show_footer_nav", &true);

    tera_context.insert("prev_title", &prev_detail);
    tera_context.insert("next_title", &next_detail);

    Ok(Box::new(html(tera.render("ci-detail.html", &tera_context).unwrap())))
}

pub fn get_ci_history(test: String) -> Box<dyn Reply> {
    handler(get_ci_history_(test))
}

fn get_ci_history_(test_hash: String) -> RawResponse {
    let tera = &TERA;
    let mut tera_context = TeraContextBuilder {
        image_modal_box: false,
        markdown: true,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec!["/ci-page.css"],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();

    if test_hash.starts_with(".") || test_hash.contains("/") {
        return Err((400, format!("malicious input for `get_ci_history`: {test_hash:?}")));
    }

    let detail = read_string(
        &join3(
            "test-results",
            "history",
            &format!("{test_hash}.json"),
        ).handle_error(400)?,
    ).handle_error(404)?;
    let detail = serde_json::from_str::<CiHistoryDetail>(&detail).handle_error(500)?;
    let cases = ci::into_renderables(&detail.tests, true).handle_error(500)?;

    tera_context.insert("title", &detail.meta.name);
    tera_context.insert("description", &detail.meta.description);
    tera_context.insert("detail", &detail);
    tera_context.insert("cases", &cases);

    tera_context.insert("show_description", &true);
    tera_context.insert("show_download_button", &false);
    tera_context.insert("show_git_info", &false);
    tera_context.insert("show_metadata", &false);
    tera_context.insert("show_footer_nav", &false);

    Ok(Box::new(html(tera.render("ci-detail.html", &tera_context).unwrap())))
}

pub fn download_json(title: String) -> Box<dyn Reply> {
    handler(download_json_(title))
}

fn download_json_(title: String) -> RawResponse {
    if title.contains(".") || title.contains("/") {
        return Err((400, format!("malicious input for `download_json`: {title:?}")));
    }

    let j = read_string(
        &join(
            "test-results",
            &format!("result-{title}.json"),
        ).handle_error(400)?,
    ).handle_error(404)?;

    Ok(Box::new(with_header(
        j,
        "Content-Disposition",
        format!("attachment; filename=\"result-{title}.json\""),
    )))
}
