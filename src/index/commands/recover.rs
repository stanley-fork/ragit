use super::Index;
use crate::{ApiConfig, BuildConfig, QueryConfig, chunk};
use crate::chunk::ChunkSource;
use crate::constant::{FILE_INDEX_DIR_NAME, INDEX_DIR_NAME};
use crate::error::Error;
use crate::index::{
    IIStatus,
    tfidf,
};
use crate::uid::{self, Uid, UidWriteMode};
use ragit_fs::{
    WriteMode,
    create_dir_all,
    exists,
    join3,
    parent,
    read_dir,
    read_string,
    remove_dir_all,
    remove_file,
    set_extension,
    try_create_dir,
    write_bytes,
};
use std::collections::HashMap;
use std::fmt;

pub type Path = String;

#[derive(Clone, Debug)]
pub struct RecoverResult {
    removed_chunks: usize,
    created_tfidfs: usize,
    replaced_configs: Vec<String>,
    staged_files: Vec<String>,
}

impl RecoverResult {
    pub fn is_empty(&self) -> bool {
        self.removed_chunks == 0
        && self.created_tfidfs == 0
        && self.replaced_configs.is_empty()
        && self.staged_files.is_empty()
    }
}

impl fmt::Display for RecoverResult {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            fmt,
            "{} chunks removed, {} tfidfs created, {} configs replaced, {} files staged",
            self.removed_chunks,
            self.created_tfidfs,
            self.replaced_configs.len(),
            self.staged_files.len(),
        )
    }
}

impl Index {
    /// This is `recover` of `rag check --recover`. It tries its best to make the index usable.
    ///
    /// - Recover A: It creates file_indexes from scratch.
    /// - Recover B: If a chunk belongs to a file that's not in self.processed_files, it's removed.
    ///     - Recover B-1: If a chunk points to a chunk that does not exist, the chunk is removed (gc).
    /// - Recover C: If there's a broken tfidf file, it creates a new one.
    /// - Recover D: If there's a broken config file, it replaces the file with a default one.
    /// - Recover E: If self.curr_processing_file is not None, the file is staged.
    pub fn recover(&mut self) -> Result<RecoverResult, Error> {
        let mut processed_files: HashMap<Path, Vec<(Uid, usize)>> = HashMap::new();
        let mut chunk_count = 0;
        let mut result = RecoverResult {
            removed_chunks: 0,
            created_tfidfs: 0,
            replaced_configs: vec![],
            staged_files: vec![],
        };

        for chunk_file in self.get_all_chunk_files()? {
            let chunk_ = chunk::load_from_file(&chunk_file)?;
            let tfidf_file = set_extension(&chunk_file, "tfidf")?;

            match &chunk_.source {
                ChunkSource::File { path, .. } => {
                    if !self.processed_files.contains_key(path) {
                        // Recover B
                        remove_file(&chunk_file)?;

                        if exists(&tfidf_file) {
                            remove_file(&tfidf_file)?;
                        }

                        result.removed_chunks += 1;
                        continue;
                    }
                },
            }

            let corrupted_tfidf_file = !exists(&tfidf_file) || tfidf::load_from_file(&tfidf_file).is_err();

            if corrupted_tfidf_file {
                chunk::save_to_file(
                    &chunk_file,
                    &chunk_,
                    self.build_config.compression_threshold,
                    self.build_config.compression_level,
                    &self.root_dir,
                    true,  // create tfidf
                )?;
                result.created_tfidfs += 1;
            }

            if let ChunkSource::File { path, index, page: _ } = &chunk_.source {
                match processed_files.get_mut(path) {
                    Some(chunks) => {
                        chunks.push((chunk_.uid, *index));
                    },
                    None => {
                        processed_files.insert(path.clone(), vec![(chunk_.uid, *index)]);
                    },
                }
            }

            chunk_count += 1;
        }

        // Recover A
        let file_index_path = join3(
            &self.root_dir,
            &INDEX_DIR_NAME,
            &FILE_INDEX_DIR_NAME,
        )?;

        if !exists(&file_index_path) {
            create_dir_all(&file_index_path)?;
        }

        for dir in read_dir(&file_index_path, false)? {
            remove_dir_all(&dir)?;
        }

        for (file, mut chunks) in processed_files.into_iter() {
            chunks.sort_by_key(|(_, index)| *index);

            if chunks[0].1 != 0 {
                return Err(Error::BrokenIndex(format!("The first chunk of `{file}` is missing.")));
            }

            // There may be multiple chunks with the same index (https://github.com/baehyunsol/ragit/issues/8), (https://github.com/baehyunsol/ragit/issues/9).
            // In such cases, it keeps only one of them and remove the others.
            chunks.dedup_by_key(|(_, index)| *index);

            if chunks[chunks.len() - 1].1 != chunks.len() - 1 {
                return Err(Error::BrokenIndex(format!("Some chunks of `{file}` is missing.")));
            }

            let file_uid = self.processed_files.get(&file).unwrap();
            let file_index_path = Index::get_uid_path(
                &self.root_dir,
                FILE_INDEX_DIR_NAME,
                *file_uid,
                None,
            )?;
            let parent_path = parent(&file_index_path)?;

            if !exists(&parent_path) {
                try_create_dir(&parent_path)?;
            }

            // Recover A
            uid::save_to_file(
                &file_index_path,
                &chunks.iter().map(|(uid, _)| *uid).collect::<Vec<_>>(),
                UidWriteMode::Naive,
            )?;
        }

        // Recover E
        if self.curr_processing_file.is_some() {
            self.curr_processing_file = None;
        }

        self.chunk_count = chunk_count;

        // Recover D
        let reset_build_config = match read_string(&self.get_build_config_path()?) {
            Ok(j) => serde_json::from_str::<BuildConfig>(&j).is_err(),
            _ => true,
        };

        if reset_build_config {
            write_bytes(
                &self.get_build_config_path()?,
                &serde_json::to_vec_pretty(&BuildConfig::default())?,
                WriteMode::Atomic,
            )?;
            result.replaced_configs.push(String::from("build"));
        }

        let reset_query_config = match read_string(&self.get_query_config_path()?) {
            Ok(j) => serde_json::from_str::<QueryConfig>(&j).is_err(),
            _ => true,
        };

        if reset_query_config {
            write_bytes(
                &self.get_query_config_path()?,
                &serde_json::to_vec_pretty(&QueryConfig::default())?,
                WriteMode::Atomic,
            )?;
            result.replaced_configs.push(String::from("query"));
        }

        let reset_api_config = match read_string(&self.get_api_config_path()?) {
            Ok(j) => serde_json::from_str::<ApiConfig>(&j).is_err(),
            _ => true,
        };

        if reset_api_config {
            write_bytes(
                &self.get_api_config_path()?,
                &serde_json::to_vec_pretty(&ApiConfig::default())?,
                WriteMode::Atomic,
            )?;
            result.replaced_configs.push(String::from("api"));
        }

        if (result.removed_chunks > 0 || result.created_tfidfs > 0) && self.ii_status != IIStatus::None {
            self.ii_status = IIStatus::Outdated;
        }

        self.calculate_and_save_uid()?;
        self.save_to_file()?;
        Ok(result)
    }
}
