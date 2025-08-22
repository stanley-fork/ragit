use async_std::task;
use crate::error::Error;
use crate::models::{TopMenu, fetch_repositories};
use crate::utils::{
    fetch_text,
    int_comma,
    render_time,
    trim_long_string,
    uri_from_str,
};
use lazy_static::lazy_static;
use ragit_fs::{
    WriteMode,
    read_string,
    write_log,
    write_string,
};
use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;
use std::sync::RwLock;
use std::time::Duration;
use tera::{Context as TeraContext, Tera};
use warp::reply::{Reply, html};

mod blog;
mod chunk;
mod ci;
mod file;
mod health;
mod image;
mod index;
mod proxy;
mod repo;

pub use blog::{
    get_blog_index,
    get_blog_article,
};
pub use chunk::get_chunk;
pub use ci::{
    download_json,
    get_ci_detail,
    get_ci_history,
    get_ci_index,
};
pub use file::get_file;
pub use health::get_health;
pub use image::{
    fetch_repo_image,
    get_image_detail,
};
pub use index::{
    get_index,
    post_index,
};
pub use proxy::{Method, ProxyBuilder, ResponseType};
pub use repo::{
    get_repo_detail,
    get_repo_index,
    search_repo,
};

lazy_static! {
    pub static ref TERA: Tera = {
        let mut result = Tera::new("./templates/*").unwrap();
        result.register_filter("render_time", render_time);
        result.register_filter("int_comma", int_comma);
        result.register_filter("markdown", |value: &tera::Value, _: &HashMap<String, tera::Value>| {
            match value {
                tera::Value::String(s) => {
                    let result = mdxt::render_to_html_with_default_options(&s);
                    Ok(tera::Value::String(result))
                },
                s => Ok(tera::Value::String(s.to_string())),
            }
        });
        result
    };

    pub static ref VERSION: String = {
        match Command::new("git").args(["rev-parse", "HEAD"]).output() {
            Ok(output) => match String::from_utf8(output.stdout) {
                Ok(s) if s.len() > 9 => match s.get(0..9) {
                    Some(s) => s.to_string(),
                    None => String::from("error"),
                },
                _ => String::from("error"),
            },
            Err(_) => String::from("error"),
        }
    };
}

pub type RawResponse = Result<Box<dyn Reply>, (u16, String)>;

pub fn redirect(url: &str) -> Box<dyn Reply> {
    Box::new(warp::redirect::found(uri_from_str(url)))
}

pub fn error_page(_: u16, message: &str) -> Box<dyn Reply> {
    let tera = &TERA;
    let mut tera_context = TeraContextBuilder {
        image_modal_box: false,
        markdown: false,
        top_menu: true,
        default_top_menu: true,
        nav: true,
        show_versions: true,
        extra_styles: vec![],
        extra_scripts: vec![],
        extra_components: vec![],
    }.build();

    tera_context.insert("message", message);

    Box::new(html(tera.render("error.html", &tera_context).unwrap()))
}

pub fn handler(r: RawResponse) -> Box<dyn Reply> {
    match r {
        Ok(r) => r,
        Err((code, error)) => error_page(code, &error),
    }
}

pub trait HandleError<T> {
    fn handle_error(self, code: u16) -> Result<T, (u16, String)>;
}

impl<T, E: std::fmt::Debug> HandleError<T> for Result<T, E> {
    fn handle_error(self, code: u16) -> Result<T, (u16, String)> {
        self.map_err(|e| {
            let e = format!("{e:?}");
            write_log(
                "handle_error",
                &format!("{code}, {}", trim_long_string(&e, 200, 200)),
            );

            (code, format!("{code} Error"))
        })
    }
}

pub struct TeraContextBuilder<'a> {
    pub image_modal_box: bool,
    pub markdown: bool,
    pub top_menu: bool,
    pub default_top_menu: bool,
    pub nav: bool,
    pub show_versions: bool,
    pub extra_styles: Vec<&'a str>,
    pub extra_scripts: Vec<&'a str>,
    pub extra_components: Vec<&'a str>,
}

impl<'a> TeraContextBuilder<'a> {
    pub fn build(&mut self) -> TeraContext {
        let mut tera_context = TeraContext::new();
        let mut styles = vec!["/style.css"];
        let mut scripts = vec![];
        let mut components = vec![];

        if self.markdown {
            styles.push("/markdown.css");
        }

        if self.image_modal_box {
            styles.push("/image-modal-box.css");
            scripts.push("/image-modal-box.js");
            components.push(include_str!("../components/image-modal-box.html"));
        }

        if self.default_top_menu {
            self.top_menu = true;
            tera_context.insert(
                "top_menu",
                &TopMenu::new(vec![
                    ("Home", "/"),
                    ("Explore", "/list"),
                    ("Blog", "/blog"),
                    ("CI", "/ci"),
                ],
            ));
        }

        if self.top_menu {
            styles.push("/top-menu.css");
            scripts.push("/top-menu.js");
        }

        if self.nav {
            styles.push("/nav.css");
            components.push(include_str!("../components/nav.html"));
        }

        if self.show_versions {
            tera_context.insert("ragithub_version", &VERSION.to_string());
        }

        for style in self.extra_styles.iter() {
            styles.push(style);
        }

        for script in self.extra_scripts.iter() {
            scripts.push(script);
        }

        for component in self.extra_components.iter() {
            components.push(component);
        }

        tera_context.insert("styles", &styles);
        tera_context.insert("scripts", &scripts);
        tera_context.insert("components", &components);
        tera_context
    }
}

impl<'a> Default for TeraContextBuilder<'a> {
    fn default() -> Self {
        TeraContextBuilder {
            image_modal_box: false,
            markdown: false,
            top_menu: false,
            default_top_menu: true,
            nav: true,
            show_versions: true,
            extra_styles: vec![],
            extra_scripts: vec![],
            extra_components: vec![],
        }
    }
}

pub(crate) fn get_or<T: FromStr>(query: &HashMap<String, String>, key: &str, default_value: T) -> T {
    match query.get(key) {
        // many clients use an empty string to represent a null value
        Some(v) if v.is_empty() => default_value,

        Some(v) => match v.parse::<T>() {
            Ok(v) => v,
            Err(_) => default_value,
        },
        None => default_value,
    }
}

lazy_static! {
    static ref BACKEND: RwLock<String> = RwLock::new(String::new());
}

pub fn get_backend() -> String {
    match BACKEND.try_read() {
        Ok(backend) => backend.to_string(),
        Err(e) => {
            write_log(
                "get_backend",
                &format!("`BACKEND.try_read()` returned {e:?}"),
            );

            // this will raise an error later
            String::from("err")
        },
    }
}

pub fn set_backend(s: &str) {
    match BACKEND.try_write() {
        Ok(mut b) => {
            *b = s.to_string();
        },
        Err(e) => {
            write_log(
                "set_backend",
                &format!("`BACKEND.try_write()` returned {e:?}"),
            );

            // It's only called once, so it's okay to panic.
            panic!()
        },
    }
}

// It runs every 5 minutes in the background.
// It's usually used to create local caches.
pub async fn background_worker() {
    loop {
        write_log(
            "background_worker",
            "woke up!",
        );

        if let Err(e) = fetch_repositories().await {
            write_log(
                "fetch_repositories",
                &format!("{e:?}"),
            );
        }

        write_log(
            "background_worker",
            "going back to sleep...",
        );
        task::sleep(Duration::from_secs(300)).await;
    }
}
