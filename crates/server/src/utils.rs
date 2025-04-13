use bytes::BufMut;
use crate::CONFIG;
use crate::error::Error;
use futures_util::TryStreamExt;
use ragit_fs::{FileError, join4};
use std::collections::HashMap;
use warp::filters::multipart::FormData;

pub async fn fetch_form_data(form: FormData) -> Result<HashMap<String, Vec<u8>>, Error> {
    Ok(form.and_then(|mut field| async move {
        let mut buffer = Vec::new();

        while let Some(content) = field.data().await {
            buffer.put(content?);
        }

        Ok((field.name().to_string(), buffer))
    }).try_collect().await?)
}

// ROOT/{user}/{repo}/.ragit
pub fn get_rag_path(user: &str, repo: &str) -> Result<String, FileError> {
    let config = CONFIG.get().unwrap();
    join4(
        &config.repo_data_dir,
        user,
        repo,
        ".ragit",
    )
}

pub fn trim_long_string(s: &str, prefix_len: usize, suffix_len: usize) -> String {
    if s.len() <= (prefix_len + suffix_len) || s.chars().count() <= (prefix_len + suffix_len) {
        s.to_string()
    }

    else {
        format!(
            "{}...{}",
            s.chars().take(prefix_len).collect::<String>(),
            s.chars().rev().take(suffix_len).collect::<String>().chars().rev().collect::<String>(),
        )
    }
}

pub fn normalize(name: &str) -> String {
    let mut chs = vec![];

    for ch in name.chars() {
        match ch {
            '0'..='9'
            | 'a'..='z'
            | 'A'..='Z'
            | '_' | '-' => { chs.push(ch); },
            _ => {},
        }
    }

    chs.into_iter().collect()
}
