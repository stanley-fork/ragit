use crate::INDEX_DIR_NAME;
use crate::chunk::CHUNK_DIR_NAME;
use crate::error::Error;
use crate::index::{FILE_INDEX_DIR_NAME, Index};
use lazy_static::lazy_static;
use ragit_fs::{
    extension,
    file_name,
    is_dir,
    join3,
    join4,
    read_dir,
};
use regex::Regex;

pub type Uid = String;
pub type Path = String;

lazy_static! {
    static ref UID_RE: Regex = Regex::new(r"^([0-9a-z]{1,64})$").unwrap();
}

impl Index {
    pub fn get_all_chunk_uids(&self) -> Result<Vec<Uid>, Error> {
        let mut result = vec![];

        for internal in read_dir(&join3(&self.root_dir, &INDEX_DIR_NAME, &CHUNK_DIR_NAME)?)? {
            let prefix = file_name(&internal)?;

            if !is_dir(&internal) {
                continue;
            }

            for chunk_file in read_dir(&internal)? {
                if extension(&chunk_file).unwrap_or(None).unwrap_or(String::new()) == "chunk" {
                    result.push(format!("{prefix}{}", file_name(&chunk_file)?));
                }
            }
        }

        Ok(result)
    }

    pub fn get_all_file_uids(&self) -> Vec<Uid> {
        self.processed_files.values().map(|uid| uid.to_string()).collect()
    }

    /// General purpose uid query for many commands: `ls-chunks`, `ls-files`, `tfidf --show` ...
    ///
    /// It first queries chunk uids and file uids that starts with `q`.
    /// If no uid's found, it treats `q` like a file path and tries to
    /// find a file uid of a file who has the uid. It doesn't do a
    /// prefix-matching when querying file paths.
    pub fn uid_query(&self, q: &str) -> Result<UidQueryResult, Error> {
        if q.is_empty() {
            return Ok(UidQueryResult::NoMatch);
        }

        let mut matched_chunk_uids = vec![];
        let mut matched_file_uids = vec![];

        if UID_RE.is_match(q) {
            if q.len() == 1 {
                for chunk_dir in read_dir(&join3(
                    &self.root_dir,
                    INDEX_DIR_NAME,
                    CHUNK_DIR_NAME,
                )?).unwrap_or(vec![]) {
                    let chunk_prefix = file_name(&chunk_dir)?;

                    if chunk_prefix.starts_with(q) {
                        for chunk_file in read_dir(&chunk_dir)? {
                            if extension(&chunk_file)?.unwrap_or(String::new()) != "chunk" {
                                continue;
                            }

                            matched_chunk_uids.push(format!("{chunk_prefix}{}", file_name(&chunk_file)?));
                        }
                    }
                }

                for file_index_dir in read_dir(&join3(
                    &self.root_dir,
                    INDEX_DIR_NAME,
                    FILE_INDEX_DIR_NAME,
                )?).unwrap_or(vec![]) {
                    let file_index_prefix = file_name(&file_index_dir)?;

                    if file_index_prefix.starts_with(q) {
                        for file_index in read_dir(&file_index_dir)? {
                            matched_file_uids.push(format!("{file_index_prefix}{}", file_name(&file_index)?));
                        }
                    }
                }
            }

            else if q.len() == 2 {
                for chunk_file in read_dir(&join4(
                    &self.root_dir,
                    INDEX_DIR_NAME,
                    CHUNK_DIR_NAME,
                    &q,
                )?).unwrap_or(vec![]) {
                    if extension(&chunk_file)?.unwrap_or(String::new()) != "chunk" {
                        continue;
                    }

                    matched_chunk_uids.push(format!("{q}{}", file_name(&chunk_file)?));
                }

                for file_index in read_dir(&join4(
                    &self.root_dir,
                    INDEX_DIR_NAME,
                    FILE_INDEX_DIR_NAME,
                    &q,
                )?).unwrap_or(vec![]) {
                    matched_file_uids.push(format!("{q}{}", file_name(&file_index)?));
                }
            }

            else {
                let prefix = q.get(0..2).unwrap().to_string();
                let suffix = q.get(2..).unwrap().to_string();

                for chunk_file in read_dir(&join4(
                    &self.root_dir,
                    INDEX_DIR_NAME,
                    CHUNK_DIR_NAME,
                    &prefix,
                )?).unwrap_or(vec![]) {
                    if extension(&chunk_file)?.unwrap_or(String::new()) != "chunk" {
                        continue;
                    }

                    let chunk_file = file_name(&chunk_file)?;

                    if chunk_file.starts_with(&suffix) {
                        matched_chunk_uids.push(format!("{prefix}{chunk_file}"));
                    }
                }

                for file_index in read_dir(&join4(
                    &self.root_dir,
                    INDEX_DIR_NAME,
                    FILE_INDEX_DIR_NAME,
                    &prefix,
                )?).unwrap_or(vec![]) {
                    let file_index = file_name(&file_index)?;

                    if file_index.starts_with(&suffix) {
                        matched_file_uids.push(format!("{prefix}{file_index}"));
                    }
                }
            }
        }

        println!("{:?}", (matched_chunk_uids.len(), matched_file_uids.len()));
        match (matched_chunk_uids.len(), matched_file_uids.len()) {
            (0, 0) => {
                println!("{:?}", Index::get_rel_path(&self.root_dir, &q.to_string()));
                if let Ok(rel_path) = Index::get_rel_path(&self.root_dir, &q.to_string()) {
                    if let Some(file_uid) = self.processed_files.get(&rel_path) {
                        return Ok(UidQueryResult::FilePath { path: rel_path, uid: file_uid.to_string() });
                    }

                    if self.staged_files.contains(&rel_path) {
                        return Ok(UidQueryResult::StagedFile { path: rel_path });
                    }
                }

                Ok(UidQueryResult::NoMatch)
            },
            (1, 0) => Ok(UidQueryResult::Chunk { uid: matched_chunk_uids[0].clone() }),
            (0, 1) => Ok(UidQueryResult::FileUid { uid: matched_file_uids[0].clone() }),
            _ => Ok(UidQueryResult::Multiple {
                chunk: matched_chunk_uids,
                file: matched_file_uids,
            }),
        }
    }
}

pub enum UidQueryResult {
    NoMatch,
    Chunk { uid: Uid },
    Multiple {
        file: Vec<Uid>,
        chunk: Vec<Uid>,
    },

    /// If a query is matched by uid, it's `FileUid`.
    /// If a query is matched by file path, it's `FilePath`.
    /// Both are for processed_files.
    FileUid { uid: Uid },

    /// If a query is matched by uid, it's `FileUid`.
    /// If a query is matched by file path, it's `FilePath`.
    /// Both are for processed_files.
    FilePath { path: Path, uid: Uid },

    /// A staged file doesn't have a uid yet.
    StagedFile { path: Path },
}
