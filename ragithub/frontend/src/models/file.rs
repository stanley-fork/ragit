use async_recursion::async_recursion;
use crate::error::Error;
use crate::methods::get_backend;
use crate::utils::{fetch_json, url_encode_strict};
use ragit_fs::{
    basename,
    write_log,
};
pub use ragit_server::models::file::{FileDetail, FileType};
use serde::Serialize;
use std::collections::{hash_map, HashSet};
use std::hash::Hasher;

// ragit backend's file api returns `FileDetail`.
// the frontend first converts `FileDetail` to `Vec<RenderableFile>`, then converts it to
// `Vec<FileEntry>`, which tera can understand.

#[derive(Clone, Debug)]
pub struct RenderableFile {
    path: String,

    // `d` of `a/b/c/d`
    name: String,
    r#type: FileType,
    children: Option<Vec<RenderableFile>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileEntry {
    anchor: Option<String>,
    pre_line: Option<String>,
    post_line: Option<String>,
    prefix: String,
    r#type: String,
    href: String,
    name: String,
}

// Github says "Sorry, we had to truncate this directory to 1,000 files. 2708 entries were omitted from the list."
pub const FILE_VIEWER_LIMIT: usize = 200;

#[async_recursion(Sync)]
pub async fn fetch_files(
    path: &str,  // dir
    repo: &str,
    expand: &HashSet<String>,
    exceeded_limit: &mut bool,
) -> Result<Vec<RenderableFile>, Error> {
    let backend = get_backend();
    let files = fetch_json::<FileDetail>(
        &format!(
            "{backend}/sample/{repo}/file-content?path={}&limit={}",
            url_encode_strict(path),
            FILE_VIEWER_LIMIT + 1,
        ),
        &None,
    ).await?;
    let mut result = vec![];

    if let Some(children) = files.children {
        for child in children.iter() {
            match child.r#type {
                FileType::File => {
                    result.push(RenderableFile {
                        path: child.path.clone(),
                        name: basename(&child.path)?,
                        r#type: FileType::File,
                        children: None,
                    });
                },
                FileType::Directory => {
                    let path_hash = short_hash(&child.path);
                    let children = if expand.contains(&path_hash) {
                        Some(fetch_files(&child.path, repo, expand, exceeded_limit).await?)
                    }

                    else {
                        None
                    };

                    result.push(RenderableFile {
                        path: child.path.clone(),
                        name: basename(&child.path)?,
                        r#type: FileType::Directory,
                        children,
                    });
                },
            }
        }
    }

    else {
        write_log(
            "fetch_files",
            &format!("fetch_files({path:?}, ...) -> `{path:?}` is supposed to be a directory, but it's not."),
        );
    }

    result.sort_by_key(
        // directories come before files
        |r| format!("{}-{}", if let FileType::File = r.r#type { "f" } else { "d" }, r.name)
    );

    if result.len() > FILE_VIEWER_LIMIT {
        *exceeded_limit = true;
        result = result[..FILE_VIEWER_LIMIT].to_vec();
    }

    Ok(result)
}

pub fn render_file_entries(
    repo: &str,
    files: &[RenderableFile],
    stack: Vec<bool>,
    expand: &HashSet<String>,
) -> Vec<FileEntry> {
    let mut result = vec![];
    let expand_str = expand.iter().map(|e| e.to_string()).collect::<Vec<_>>().concat();

    for (index, file) in files.iter().enumerate() {
        let mut pre_line = stack.iter().map(
            |s| if *s {
                String::from("|   ")
            } else {
                String::from("    ")
            }
        ).collect::<Vec<String>>();
        pre_line.push(String::from("|   "));
        let pre_line = Some(pre_line.concat());

        let prefix = format!(
            "{}*-- ",
            stack.iter().map(
                |s| if *s {
                    String::from("|   ")
                } else {
                    String::from("    ")
                }
            ).collect::<Vec<String>>().concat(),
        );

        match file.r#type {
            FileType::File => {
                result.push(FileEntry {
                    anchor: None,
                    pre_line,
                    post_line: None,
                    prefix,
                    r#type: String::from("file"),
                    href: format!("/sample/{repo}/file?path={}", url_encode_strict(&file.path)),
                    name: file.name.clone(),
                });
            },
            FileType::Directory => {
                let path_hash = short_hash(&file.path);
                let href = if file.children.is_none() {
                    // expand
                    format!("?expand={expand_str}{path_hash}#{path_hash}")
                } else {
                    // already expanded
                    let expand_str = expand.iter().filter(
                        |e| **e != path_hash
                    ).map(
                        |e| e.to_string()
                    ).collect::<Vec<_>>().concat();
                    format!("?expand={expand_str}#{path_hash}")
                };

                result.push(FileEntry {
                    anchor: Some(path_hash),
                    pre_line,
                    post_line: None,
                    prefix,
                    r#type: String::from("directory"),
                    href,
                    name: file.name.clone(),
                });

                if let Some(children) = &file.children {
                    let mut stack = stack.clone();
                    stack.push(index + 1 != files.len());
                    result.append(&mut render_file_entries(repo, &children, stack, expand));
                }
            },
        }
    }

    result
}

fn short_hash(path: &str) -> String {
    let mut hasher = hash_map::DefaultHasher::new();
    hasher.write(path.as_bytes());
    let mut hash = hasher.finish();
    let mut result = vec![];

    // uses roughly 36 bits
    for _ in 0..7 {
        let b = hash % 36;

        match b {
            0..=9 => { result.push(b as u8 + b'0'); },
            10.. => { result.push(b as u8 - 10 + b'a'); },
        }

        hash /= 36;
    }

    String::from_utf8(result).unwrap()
}
