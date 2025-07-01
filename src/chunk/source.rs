use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum ChunkSource {
    /// Built directly from a file.
    /// It's `index`th chunk of `path`.
    /// `path` is a relative path.
    File {
        path: String,
        index: usize,

        // If the chunk is from a pdf file, it tells which page the chunk is from.
        page: Option<usize>,
    },
}

impl ChunkSource {
    // this value is directly used to hash this instance
    pub fn hash_str(&self) -> String {
        match self {
            ChunkSource::File { path, index, page } => format!(
                "{path}{index}{}",
                match page {
                    Some(page) => format!("p{page}"),
                    None => String::new(),
                },
            ),
        }
    }

    pub fn set_path(&mut self, new_path: String) {
        match self {
            ChunkSource::File { path, .. } => { *path = new_path; },
        }
    }

    pub fn unwrap_index(&self) -> usize {
        match self {
            ChunkSource::File { index, .. } => *index,
        }
    }

    pub fn sortable_string(&self) -> String {
        match self {
            // It doesn't care about page numbers because
            // 1. `index` is mandatory but `page` is optional.
            // 2. `index` is guaranteed to be unique and sequential,
            //    while `page` can have arbitrary values (it's up to file readers).
            ChunkSource::File { path, index, page: _ } => format!("file: {path}-{index:09}"),
        }
    }

    pub fn render(&self) -> String {
        match self {
            ChunkSource::File { path, index, page } => format!(
                "{} chunk of {path}{}",
                // it's 0-base
                match index {
                    0 => String::from("1st"),
                    1 => String::from("2nd"),
                    2 => String::from("3rd"),
                    n => format!("{}th", n + 1),
                },
                // it's 1-base
                match page {
                    Some(page) => format!(" (page {page})"),
                    None => String::new(),
                },
            ),
        }
    }
}
