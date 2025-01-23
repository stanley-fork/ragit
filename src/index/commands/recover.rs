use super::Index;
use crate::{ApiConfigRaw, BuildConfig, QueryConfig, chunk};
use crate::chunk::{CHUNK_DIR_NAME, ChunkSource};
use crate::error::Error;
use crate::index::{
    FILE_INDEX_DIR_NAME,
    IIStatus,
    INDEX_DIR_NAME,
    tfidf,
};
use crate::uid::{self, Uid, UidType};
use ragit_fs::{
    WriteMode,
    create_dir_all,
    exists,
    join3,
    parent,
    set_extension,
    read_dir,
    read_string,
    remove_dir_all,
    remove_file,
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
                ChunkSource::Chunks { .. } => {
                    // gc will run later
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
                )?;
                result.created_tfidfs += 1;
            }

            if let ChunkSource::File { path, index } = &chunk_.source {
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

        // Recover B-1: gc
        // FIXME: it has a terrible time complexity
        'gc_loop: loop {
            for chunk_uid in self.get_all_chunk_uids()? {
                match chunk_uid.get_uid_type()? {
                    UidType::Group => {
                        let chunk_ = self.get_chunk_by_uid(chunk_uid)?;
                        let chunk_path = Index::get_uid_path(
                            &self.root_dir,
                            CHUNK_DIR_NAME,
                            chunk_uid,
                            Some("chunk"),
                        )?;

                        if let ChunkSource::Chunks { uids } = &chunk_.source {
                            for uid in uids.iter() {
                                if !self.check_chunk_by_uid(*uid) {
                                    remove_file(&chunk_path)?;
                                    result.removed_chunks += 1;
                                    chunk_count -= 1;
                                    continue 'gc_loop;
                                }
                            }
                        }
                    },
                    _ => {},
                }
            }

            break;
        }

        // Recover A
        for dir in read_dir(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME,
            &FILE_INDEX_DIR_NAME,
        )?, false)? {
            remove_dir_all(&dir)?;
        }

        for (file, mut chunks) in processed_files.into_iter() {
            chunks.sort_by_key(|(_, index)| *index);
            let file_uid = self.processed_files.get(&file).unwrap();
            let file_index_path = Index::get_uid_path(
                &self.root_dir,
                FILE_INDEX_DIR_NAME,
                *file_uid,
                None,
            )?;
            let parent_path = parent(&file_index_path)?;

            if !exists(&parent_path) {
                create_dir_all(&parent_path)?;
            }

            // Recover A
            uid::save_to_file(
                &file_index_path,
                &chunks.iter().map(|(uid, _)| *uid).collect::<Vec<_>>(),
            )?;
        }

        // Recover E
        if let Some(curr_processing_file) = &self.curr_processing_file {
            // a file can be processed and staged at the same time, so let's be careful not to
            // stage the same file twice
            if !self.staged_files.contains(curr_processing_file) {
                self.staged_files.push(curr_processing_file.clone());
                result.staged_files.push(curr_processing_file.clone());
                self.curr_processing_file = None;
            }
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
                WriteMode::CreateOrTruncate,
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
                WriteMode::CreateOrTruncate,
            )?;
            result.replaced_configs.push(String::from("query"));
        }

        let reset_api_config = match read_string(&self.get_api_config_path()?) {
            Ok(j) => match serde_json::from_str::<ApiConfigRaw>(&j) {
                Ok(api_config_raw) => self.init_api_config(&api_config_raw).is_err(),
                _ => true,
            },
            _ => true,
        };

        if reset_api_config {
            write_bytes(
                &self.get_api_config_path()?,
                &serde_json::to_vec_pretty(&ApiConfigRaw::default())?,
                WriteMode::CreateOrTruncate,
            )?;
            result.replaced_configs.push(String::from("api"));
        }

        if (result.removed_chunks > 0 || result.created_tfidfs > 0) && self.ii_status != IIStatus::None {
            self.ii_status = IIStatus::Outdated;
        }

        Ok(result)
    }
}
