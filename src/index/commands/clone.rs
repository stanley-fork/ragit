use super::Index;
use crate::error::Error;
use ragit_fs::{
    create_dir,
    remove_dir_all,
};
use reqwest::Url;

pub struct CloneResult {
    url: String,
    image_total: usize,
    image_count: usize,
    chunk_total: usize,
    chunk_count: usize,
}

impl Index {
    pub async fn clone(url: String, repo_name: Option<String>) -> Result<CloneResult, Error> {
        let repo_name = repo_name.unwrap_or_else(|| infer_repo_name_from_url(&url));
        create_dir(&repo_name)?;

        match Index::clone_worker(url, repo_name.clone()).await {
            Ok(result) => Ok(result),
            Err(e) => {
                remove_dir_all(&repo_name)?;
                Err(e)
            },
        }
    }

    async fn clone_worker(url: String, repo_name: String) -> Result<CloneResult, Error> {
        todo!()
    }

    // TODO: erase lines instead of the entire screen
    fn render_clone_dashboard(state: &CloneResult) {
        todo!()
    }
}

// TODO: it's too naive
fn infer_repo_name_from_url(url: &str) -> String {
    // This function doesn't need any error-handling
    // because if any of these fail, `Index::clone_worker()`
    // would also fail and there's an error handler for
    // `Index::clone_worker()`.
    match Url::parse(url) {
        Ok(url) => match url.path_segments() {
            Some(paths) => match paths.last() {
                Some(name) => name.to_string(),
                _ => String::from("_"),
            },
            _ => String::from("_"),
        },
        _ => String::from("_"),
    }
}
