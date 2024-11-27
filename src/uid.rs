use crate::INDEX_DIR_NAME;
use crate::chunk::CHUNK_DIR_NAME;
use crate::error::Error;
use crate::index::{FILE_INDEX_DIR_NAME, Index};
use lazy_static::lazy_static;
use ragit_fs::{
    WriteMode,
    extension,
    file_name,
    is_dir,
    join3,
    join4,
    read_bytes,
    read_dir,
    write_string,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Each chunk and file has uid.
///
/// Uid is a 256 bit hash value, generated by sha3-256 hash function and some postprocessing.
/// The most convenient way for users to deal with uid is using `uid_query` function. The user
/// inputs a hex representation of a uid, or a prefix of it, and the function returns
/// matched uids.
///
/// File uid and chunk uid has a small difference and ragit uses the difference to distinguish
/// chunks and files. When a file uid is represented in hexadecimal, the last 12 characters of the hex
/// are always numbers (no alphabets). And the numbers represents the length of the file, in bytes.
/// For a chunk uid, it's guaranteed that at least one character of the last 12 characters is an
/// alphabet.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Uid {
    high: u128,
    low: u128,
}

pub type Path = String;

lazy_static! {
    static ref UID_RE: Regex = Regex::new(r"^([0-9a-z]{1,64})$").unwrap();
}

// `Vec<Uid>` has multiple serialization formats, though only 1 is implemented now.
// Format 1, naive format: store hex representations of uids, using newline character as a delimiter.
// Format 2, compact format: TODO
pub fn load_from_file(path: &str) -> Result<Vec<Uid>, Error> {
    let bytes = read_bytes(path)?;

    match bytes.get(0) {
        Some((b'a'..=b'f') | (b'0'..=b'9')) => match String::from_utf8(bytes) {
            Ok(s) => {
                let mut result = vec![];

                for line in s.lines() {
                    result.push(line.parse::<Uid>()?);
                }

                Ok(result)
            },
            Err(_) => Err(Error::CorruptedFile(path.to_string())),
        },
        Some(b) => Err(Error::CorruptedFile(path.to_string())),
        None => Ok(vec![]),
    }
}

// For now, it only supports naive format.
pub fn save_to_file(
    path: &str,
    uids: &[Uid],
) -> Result<(), Error> {
    Ok(write_string(
        path,
        &uids.iter().map(|uid| uid.to_string()).collect::<Vec<_>>().join("\n"),
        WriteMode::CreateOrTruncate,
    )?)
}

impl Uid {
    pub(crate) fn dummy() -> Self {
        Uid {
            high: 0,
            low: 0,
        }
    }

    pub(crate) fn from_prefix_and_suffix(prefix: &str, suffix: &str) -> Result<Self, Error> {
        if prefix.len() != 2 || suffix.len() != 62 {
            Err(Error::InvalidUid(format!("{prefix}{suffix}")))
        }

        else {
            match (suffix.get(0..30), suffix.get(30..)) {
                (Some(high_suff), Some(low)) => match (
                    u128::from_str_radix(&format!("{prefix}{high_suff}"), 16),
                    u128::from_str_radix(low, 16),
                ) {
                    (Ok(high), Ok(low)) => Ok(Uid { high, low }),
                    _ => Err(Error::InvalidUid(format!("{prefix}{suffix}"))),
                },
                _ => Err(Error::InvalidUid(format!("{prefix}{suffix}"))),
            }
        }
    }

    pub(crate) fn get_prefix(&self) -> String {
        format!("{:02x}", self.high >> 120)
    }

    pub(crate) fn get_suffix(&self) -> String {
        format!("{:030x}{:032x}", self.high & 0xff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, self.low)
    }

    pub(crate) fn get_file_size(&self) -> Result<usize, Error> {
        let low = format!("{:x}", self.low & 0xffff_ffff_ffff);
        low.parse::<usize>().map_err(|e| Error::InvalidUid(self.to_string()))
    }
}

impl fmt::Display for Uid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "{:032x}{:032x}", self.high, self.low)
    }
}

impl FromStr for Uid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        if s.len() != 64 {
            Err(Error::InvalidUid(s.to_string()))
        }

        else {
            match (s.get(0..32), s.get(32..)) {
                (Some(high), Some(low)) => match (
                    u128::from_str_radix(high, 16),
                    u128::from_str_radix(low, 16),
                ) {
                    (Ok(high), Ok(low)) => Ok(Uid { high, low }),
                    _ => Err(Error::InvalidUid(s.to_string())),
                },
                _ => Err(Error::InvalidUid(s.to_string())),
            }
        }
    }
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
                    result.push(Uid::from_prefix_and_suffix(&prefix, &file_name(&chunk_file)?)?);
                }
            }
        }

        Ok(result)
    }

    pub fn get_all_file_uids(&self) -> Vec<Uid> {
        self.processed_files.values().map(|uid| *uid).collect()
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

                            matched_chunk_uids.push(Uid::from_prefix_and_suffix(&chunk_prefix, &file_name(&chunk_file)?)?);
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
                            matched_file_uids.push(Uid::from_prefix_and_suffix(&file_index_prefix, &file_name(&file_index)?)?);
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

                    matched_chunk_uids.push(Uid::from_prefix_and_suffix(&q, &file_name(&chunk_file)?)?);
                }

                for file_index in read_dir(&join4(
                    &self.root_dir,
                    INDEX_DIR_NAME,
                    FILE_INDEX_DIR_NAME,
                    &q,
                )?).unwrap_or(vec![]) {
                    matched_file_uids.push(Uid::from_prefix_and_suffix(&q, &file_name(&file_index)?)?);
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
                        matched_chunk_uids.push(Uid::from_prefix_and_suffix(&prefix, &chunk_file)?);
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
                        matched_file_uids.push(Uid::from_prefix_and_suffix(&prefix, &file_index)?);
                    }
                }
            }
        }

        match (matched_chunk_uids.len(), matched_file_uids.len()) {
            (0, 0) => {
                if let Ok(rel_path) = Index::get_rel_path(&self.root_dir, &q.to_string()) {
                    if let Some(uid) = self.processed_files.get(&rel_path) {
                        return Ok(UidQueryResult::FilePath { path: rel_path, uid: *uid });
                    }

                    if self.staged_files.contains(&rel_path) {
                        return Ok(UidQueryResult::StagedFile { path: rel_path });
                    }
                }

                Ok(UidQueryResult::NoMatch)
            },
            (1, 0) => Ok(UidQueryResult::Chunk { uid: matched_chunk_uids[0] }),
            (0, 1) => Ok(UidQueryResult::FileUid { uid: matched_file_uids[0] }),
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