use super::Uid;
use crate::constant::{
    CHUNK_DIR_NAME,
    FILE_INDEX_DIR_NAME,
    IMAGE_DIR_NAME,
    INDEX_DIR_NAME,
};
use crate::error::Error;
use crate::index::Index;
use lazy_static::lazy_static;
use ragit_fs::{
    exists,
    extension,
    file_name,
    get_relative_path,
    join,
    join3,
    join4,
    read_dir,
    set_extension,
};
use regex::Regex;
use std::collections::{HashMap, HashSet};

lazy_static! {
    // full or prefix
    static ref UID_RE: Regex = Regex::new(r"^([0-9a-z]{1,64})$").unwrap();
}

impl Index {
    /// The result is sorted by uid.
    /// Sorting 1) makes the result deterministic and 2) some functions rely on this behavior.
    pub fn get_all_file_uids(&self) -> Vec<Uid> {
        let mut result: Vec<Uid> = self.processed_files.values().map(|uid| *uid).collect();
        result.sort();
        result
    }

    pub fn uid_query(&self, qs: &[String], config: UidQueryConfig) -> Result<UidQueryResult, Error> {
        let mut chunks_set = HashSet::new();
        let mut images_set = HashSet::new();
        let mut processed_files_map = HashMap::new();
        let mut staged_files_set = HashSet::new();

        for q in qs.iter() {
            let curr = self.uid_query_unit(q, config)?;

            for chunk in curr.chunks.iter() {
                chunks_set.insert(*chunk);
            }

            for image in curr.images.iter() {
                images_set.insert(*image);
            }

            for (path, uid) in curr.processed_files.iter() {
                processed_files_map.insert(*uid, path.to_string());
            }

            for staged_file in curr.staged_files.iter() {
                staged_files_set.insert(staged_file.to_string());
            }
        }

        let mut chunks = chunks_set.into_iter().collect::<Vec<_>>();
        let mut images = images_set.into_iter().collect::<Vec<_>>();
        let mut processed_files = processed_files_map.into_iter().map(|(uid, path)| (path, uid)).collect::<Vec<_>>();
        let mut staged_files = staged_files_set.into_iter().collect::<Vec<_>>();

        // The result has to be deterministic
        chunks.sort();
        images.sort();
        processed_files.sort_by_key(|(_, uid)| *uid);
        staged_files.sort();

        Ok(UidQueryResult {
            chunks,
            images,
            processed_files,
            staged_files,
        })
    }

    fn uid_query_unit(&self, q: &str, config: UidQueryConfig) -> Result<UidQueryResult, Error> {
        if q.is_empty() {
            return Ok(UidQueryResult::empty());
        }

        let mut chunks = vec![];
        let mut images = vec![];
        let mut staged_files = vec![];

        // below 2 are for processed files
        let mut file_uids = vec![];
        let mut file_paths = vec![];

        if UID_RE.is_match(q) {
            if q.len() == 1 {
                if config.search_chunk {
                    for chunk_dir in read_dir(&join3(
                        &self.root_dir,
                        INDEX_DIR_NAME,
                        CHUNK_DIR_NAME,
                    )?, false).unwrap_or(vec![]) {
                        let chunk_prefix = file_name(&chunk_dir)?;

                        if chunk_prefix.starts_with(q) {
                            for chunk_file in read_dir(&chunk_dir, false)? {
                                if extension(&chunk_file)?.unwrap_or(String::new()) != "chunk" {
                                    continue;
                                }

                                chunks.push(Uid::from_prefix_and_suffix(&chunk_prefix, &file_name(&chunk_file)?)?);
                            }
                        }
                    }
                }

                if config.search_file_uid {
                    for file_index_dir in read_dir(&join3(
                        &self.root_dir,
                        INDEX_DIR_NAME,
                        FILE_INDEX_DIR_NAME,
                    )?, false).unwrap_or(vec![]) {
                        let file_index_prefix = file_name(&file_index_dir)?;

                        if file_index_prefix.starts_with(q) {
                            for file_index in read_dir(&file_index_dir, false)? {
                                file_uids.push(Uid::from_prefix_and_suffix(&file_index_prefix, &file_name(&file_index)?)?);
                            }
                        }
                    }
                }

                if config.search_image {
                    for image_dir in read_dir(&join3(
                        &self.root_dir,
                        INDEX_DIR_NAME,
                        IMAGE_DIR_NAME,
                    )?, false).unwrap_or(vec![]) {
                        let image_prefix = file_name(&image_dir)?;

                        if image_prefix.starts_with(q) {
                            for image_file in read_dir(&image_dir, false)? {
                                if extension(&image_file)?.unwrap_or(String::new()) != "png" {
                                    continue;
                                }

                                images.push(Uid::from_prefix_and_suffix(&image_prefix, &file_name(&image_file)?)?);
                            }
                        }
                    }
                }
            }

            else if q.len() == 2 {
                if config.search_chunk {
                    for chunk_file in read_dir(&join4(
                        &self.root_dir,
                        INDEX_DIR_NAME,
                        CHUNK_DIR_NAME,
                        q,
                    )?, false).unwrap_or(vec![]) {
                        if extension(&chunk_file)?.unwrap_or(String::new()) != "chunk" {
                            continue;
                        }

                        chunks.push(Uid::from_prefix_and_suffix(q, &file_name(&chunk_file)?)?);
                    }
                }

                if config.search_file_uid {
                    for file_index in read_dir(&join4(
                        &self.root_dir,
                        INDEX_DIR_NAME,
                        FILE_INDEX_DIR_NAME,
                        q,
                    )?, false).unwrap_or(vec![]) {
                        file_uids.push(Uid::from_prefix_and_suffix(q, &file_name(&file_index)?)?);
                    }
                }

                if config.search_image {
                    for image_file in read_dir(&join4(
                        &self.root_dir,
                        INDEX_DIR_NAME,
                        IMAGE_DIR_NAME,
                        q,
                    )?, false).unwrap_or(vec![]) {
                        if extension(&image_file)?.unwrap_or(String::new()) != "png" {
                            continue;
                        }

                        images.push(Uid::from_prefix_and_suffix(q, &file_name(&image_file)?)?);
                    }
                }
            }

            else {
                let prefix = q.get(0..2).unwrap().to_string();
                let suffix = q.get(2..).unwrap().to_string();

                if config.search_chunk {
                    if q.len() == 64 {
                        let chunk_at = join(
                            &join3(
                                &self.root_dir,
                                INDEX_DIR_NAME,
                                CHUNK_DIR_NAME,
                            )?,
                            &join(
                                &prefix,
                                &set_extension(
                                    &suffix,
                                    "chunk",
                                )?,
                            )?,
                        )?;

                        if exists(&chunk_at) {
                            chunks.push(q.parse::<Uid>()?);
                        }
                    }

                    else {
                        for chunk_file in read_dir(&join4(
                            &self.root_dir,
                            INDEX_DIR_NAME,
                            CHUNK_DIR_NAME,
                            &prefix,
                        )?, false).unwrap_or(vec![]) {
                            if extension(&chunk_file)?.unwrap_or(String::new()) != "chunk" {
                                continue;
                            }

                            let chunk_file = file_name(&chunk_file)?;

                            if chunk_file.starts_with(&suffix) {
                                chunks.push(Uid::from_prefix_and_suffix(&prefix, &chunk_file)?);
                            }
                        }
                    }
                }

                if config.search_file_uid {
                    if q.len() == 64 {
                        let file_index = join(
                            &join3(
                                &self.root_dir,
                                INDEX_DIR_NAME,
                                FILE_INDEX_DIR_NAME,
                            )?,
                            &join(
                                &prefix,
                                &suffix,
                            )?,
                        )?;

                        if exists(&file_index) {
                            file_uids.push(q.parse::<Uid>()?);
                        }
                    }

                    else {
                        for file_index in read_dir(&join4(
                            &self.root_dir,
                            INDEX_DIR_NAME,
                            FILE_INDEX_DIR_NAME,
                            &prefix,
                        )?, false).unwrap_or(vec![]) {
                            let file_index = file_name(&file_index)?;

                            if file_index.starts_with(&suffix) {
                                file_uids.push(Uid::from_prefix_and_suffix(&prefix, &file_index)?);
                            }
                        }
                    }
                }

                if config.search_image {
                    if q.len() == 64 {
                        let image_at = join(
                            &join3(
                                &self.root_dir,
                                INDEX_DIR_NAME,
                                IMAGE_DIR_NAME,
                            )?,
                            &join(
                                &prefix,
                                &set_extension(
                                    &suffix,
                                    "png",
                                )?,
                            )?,
                        )?;

                        if exists(&image_at) {
                            images.push(q.parse::<Uid>()?);
                        }
                    }

                    else {
                        for image_file in read_dir(&join4(
                            &self.root_dir,
                            INDEX_DIR_NAME,
                            IMAGE_DIR_NAME,
                            &prefix,
                        )?, false).unwrap_or(vec![]) {
                            if extension(&image_file)?.unwrap_or(String::new()) != "png" {
                                continue;
                            }

                            let image_file = file_name(&image_file)?;

                            if image_file.starts_with(&suffix) {
                                images.push(Uid::from_prefix_and_suffix(&prefix, &image_file)?);
                            }
                        }
                    }
                }
            }
        }

        if config.search_file_path {
            if let Ok(mut rel_path) = get_relative_path(&self.root_dir, q) {
                // 1. It tries to exact-match a processed file.
                if self.processed_files.contains_key(&rel_path) {
                    file_paths.push(rel_path.to_string());
                }

                // 2. It tries to exact-match a staged file.
                //    In some cases, a file can be both processed and staged at the
                //    same time. In that case, it has to choose the processed file.
                else if config.search_staged_file && self.staged_files.contains(&rel_path) {
                    staged_files.push(rel_path);
                }

                // 3. It assumes that `rel_path` is a directory and tries to
                //    find files in the directory.
                else {
                    if !rel_path.ends_with("/") && !rel_path.is_empty() {
                        rel_path = format!("{rel_path}/");
                    }

                    for path in self.processed_files.keys() {
                        if path.starts_with(&rel_path) {
                            file_paths.push(path.to_string());
                        }
                    }

                    if config.search_staged_file {
                        for staged_file in self.staged_files.iter() {
                            if staged_file.starts_with(&rel_path) {
                                staged_files.push(staged_file.to_string());
                            }
                        }
                    }
                }
            }
        }

        let mut processed_files = HashSet::with_capacity(file_paths.len() + file_uids.len());
        let processed_files_rev: HashMap<_, _> = self.processed_files.iter().map(
            |(file, uid)| (*uid, file.to_string())
        ).collect();

        for path in file_paths.iter() {
            processed_files.insert((path.to_string(), *self.processed_files.get(path).unwrap()));
        }

        for uid in file_uids.iter() {
            processed_files.insert((processed_files_rev.get(uid).unwrap().to_string(), *uid));
        }

        let processed_files: Vec<(String, Uid)> = processed_files.into_iter().collect();

        Ok(UidQueryResult {
            chunks,
            images,
            processed_files,
            staged_files,
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct UidQueryConfig {
    pub search_chunk: bool,
    pub search_image: bool,
    pub search_file_path: bool,
    pub search_file_uid: bool,

    /// It searches staged files when both `search_file_path` and `search_staged_file` are set.
    pub search_staged_file: bool,
}

impl UidQueryConfig {
    pub fn new() -> Self {
        UidQueryConfig {
            search_chunk: true,
            search_image: true,
            search_file_path: true,
            search_file_uid: true,
            search_staged_file: true,
        }
    }

    pub fn file_or_chunk_only(mut self) -> Self {
        self.search_chunk = true;
        self.search_file_path = true;
        self.search_file_uid = true;
        self.search_image = false;
        self
    }

    pub fn file_only(mut self) -> Self {
        self.search_chunk = false;
        self.search_image = false;
        self.search_file_path = true;
        self.search_file_uid = true;
        self
    }

    pub fn chunk_only(mut self) -> Self {
        self.search_chunk = true;
        self.search_image = false;
        self.search_file_path = false;
        self.search_file_uid = false;
        self
    }

    pub fn no_staged_file(mut self) -> Self {
        self.search_staged_file = false;
        self
    }
}

#[derive(Clone, Debug)]
pub struct UidQueryResult {
    pub chunks: Vec<Uid>,
    pub images: Vec<Uid>,
    pub processed_files: Vec<(String, Uid)>,
    pub staged_files: Vec<String>,
}

impl UidQueryResult {
    fn empty() -> Self {
        UidQueryResult {
            chunks: vec![],
            images: vec![],
            processed_files: vec![],
            staged_files: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn has_multiple_matches(&self) -> bool {
        self.len() > 1
    }

    pub fn len(&self) -> usize {
        self.chunks.len() + self.images.len() + self.processed_files.len() + self.staged_files.len()
    }

    pub fn get_chunk_uids(&self) -> Vec<Uid> {
        self.chunks.clone()
    }

    pub fn get_image_uids(&self) -> Vec<Uid> {
        self.images.clone()
    }

    pub fn get_file_uids(&self) -> Vec<Uid> {
        self.processed_files.iter().map(|(_, uid)| *uid).collect()
    }

    pub fn get_processed_files(&self) -> Vec<(String, Uid)> {
        self.processed_files.clone()
    }

    pub fn get_staged_files(&self) -> Vec<String> {
        self.staged_files.clone()
    }

    /// It returns `Some` iff there's only 1 match.
    pub fn get_processed_file(&self) -> Option<(String, Uid)> {
        if self.processed_files.len() == 1 {
            Some(self.processed_files[0].clone())
        }

        else {
            None
        }
    }

    /// It returns `Some` iff there's only 1 match.
    pub fn get_staged_file(&self) -> Option<String> {
        if self.staged_files.len() == 1 {
            Some(self.staged_files[0].clone())
        }

        else {
            None
        }
    }

    /// It returns `Some` iff there's only 1 match.
    pub fn get_chunk_uid(&self) -> Option<Uid> {
        if self.chunks.len() == 1 {
            Some(self.chunks[0])
        }

        else {
            None
        }
    }

    /// It returns `Some` iff there's only 1 match.
    pub fn get_image_uid(&self) -> Option<Uid> {
        if self.images.len() == 1 {
            Some(self.images[0])
        }

        else {
            None
        }
    }
}
