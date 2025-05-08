use crate::uid::Uid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum ChunkSource {
    /// Built directly from a file.
    /// It's `index`th chunk of `path`.
    /// `path` is a relative path.
    /// ``
    File {
        path: String,
        index: usize,

        // If the chunk is from a pdf file, it tells which page the chunk is from.
        page: Option<usize>,
    },

    /// TODO: There's an error with this variant: `serde_json` cannot deserialize this.
    ///       The crate can handle u128, but it seems like it cannot handle u128 inside
    ///       a tagged enum. Several issues on this are open on github, and there's even
    ///       a [fix](https://github.com/serde-rs/serde/pull/2781), but it's not merged yet.
    ///       (2025-05-08): It seems like the problem is due to `arbitrary_precision` feature.
    /// Summary of multiple chunks.
    Chunks { uids: Vec<Uid> },
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
            ChunkSource::Chunks { uids } => {
                let mut result = Uid::dummy();

                for chunk_uid in uids.iter() {
                    result ^= *chunk_uid;
                }

                result.to_string()
            },
        }
    }

    pub fn set_path(&mut self, new_path: String) {
        match self {
            ChunkSource::File { path, .. } => { *path = new_path; },
            _ => panic!(),
        }
    }

    pub fn unwrap_index(&self) -> usize {
        match self {
            ChunkSource::File { index, .. } => *index,
            _ => unreachable!(),
        }
    }

    pub fn sortable_string(&self) -> String {
        match self {
            // It doesn't care about page numbers because
            // 1. `index` is mandatory but `page` is optional.
            // 2. `index` is guaranteed to be unique and sequential,
            //    while `page` can have arbitrary values (it's up to file readers).
            ChunkSource::File { path, index, page: _ } => format!("file: {path}-{index:09}"),
            ChunkSource::Chunks { .. } => format!("chunks: {}", self.hash_str()),
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
            ChunkSource::Chunks { uids } => format!(
                "multiple chunks ({})",
                uids.iter().map(|uid| uid.to_string().get(0..8).unwrap_or("").to_string()).collect::<Vec<String>>().join(", "),
            ),
        }
    }
}

/// I added a field to `ChunkSource::File` and I'm wondering if it's backward compatible.
#[cfg(test)]
mod tests {
    use crate::Uid;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
    #[serde(tag = "type")]
    enum ChunkSourceOld {
        File {
            path: String,
            index: usize,
        },
        Chunks { uids: Vec<Uid> },
    }

    #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
    #[serde(tag = "type")]
    enum ChunkSourceNew {
        File {
            path: String,
            index: usize,
            page: Option<usize>,
        },
        Chunks { uids: Vec<Uid> },
    }

    #[test]
    #[allow(unused_variables)]
    fn serde_chunk_source() {
        let old_source1 = ChunkSourceOld::File { path: String::from("a.md"), index: 0 };
        let old_source2 = ChunkSourceOld::Chunks { uids: vec![] };
        let new_source1 = ChunkSourceNew::File { path: String::from("a.md"), index: 0, page: Some(0) };
        let new_source2 = ChunkSourceNew::File { path: String::from("a.md"), index: 0, page: None };
        let new_source3 = ChunkSourceNew::Chunks { uids: vec![] };

        let old_source1_s = serde_json::to_string(&old_source1).unwrap();
        let old_source2_s = serde_json::to_string(&old_source2).unwrap();
        let new_source1_s = serde_json::to_string(&new_source1).unwrap();
        let new_source2_s = serde_json::to_string(&new_source2).unwrap();
        let new_source3_s = serde_json::to_string(&new_source3).unwrap();

        // old version can read new versions and vice versa
        let old_source1_d_o = serde_json::from_str::<ChunkSourceOld>(&old_source1_s).unwrap();
        let old_source2_d_o = serde_json::from_str::<ChunkSourceOld>(&old_source2_s).unwrap();
        let new_source1_d_o = serde_json::from_str::<ChunkSourceOld>(&new_source1_s).unwrap();
        let new_source2_d_o = serde_json::from_str::<ChunkSourceOld>(&new_source2_s).unwrap();
        let new_source3_d_o = serde_json::from_str::<ChunkSourceOld>(&new_source3_s).unwrap();
        let old_source1_d_n = serde_json::from_str::<ChunkSourceNew>(&old_source1_s).unwrap();
        let old_source2_d_n = serde_json::from_str::<ChunkSourceNew>(&old_source2_s).unwrap();
        let new_source1_d_n = serde_json::from_str::<ChunkSourceNew>(&new_source1_s).unwrap();
        let new_source2_d_n = serde_json::from_str::<ChunkSourceNew>(&new_source2_s).unwrap();
        let new_source3_d_n = serde_json::from_str::<ChunkSourceNew>(&new_source3_s).unwrap();

        assert_eq!(old_source2_d_o, new_source3_d_o);
        assert_eq!(old_source2_d_n, new_source3_d_n);
        assert_eq!(old_source1_d_o, new_source1_d_o);
        assert_eq!(old_source1_d_o, new_source2_d_o);
    }
}
