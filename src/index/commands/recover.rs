use super::Index;
use crate::{ApiConfigRaw, BuildConfig, QueryConfig, chunk};
use crate::error::Error;
use crate::index::{
    FILE_INDEX_DIR_NAME,
    INDEX_DIR_NAME,
    tfidf,
};
use crate::uid::{self, Uid};
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

pub type Path = String;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RecoverResult {
    removed_chunk: usize,
}

impl Index {
    /// This is `recover` of `rag check --recover`. It tries its best to make the index usable.
    ///
    /// - Recover A: It creates file_indexes from scratch.
    /// - Recover B: If a chunk belongs to a file that's not in self.processed_files, it's removed.
    /// - Recover C: If there's a broken tfidf file, it creates a new one.
    /// - Recover D: If there's a broken config file, it replaces the file with a default one.
    /// - Recover E: If self.curr_processing_file is not None, the file is staged.
    pub fn recover(&mut self) -> Result<RecoverResult, Error> {
        let mut processed_files: HashMap<Path, Vec<(Uid, usize)>> = HashMap::new();
        let mut chunk_count = 0;
        let mut result = RecoverResult {
            removed_chunk: 0,
        };

        for chunk_file in self.get_all_chunk_files()? {
            let chunk_ = chunk::load_from_file(&chunk_file)?;
            let tfidf_file = set_extension(&chunk_file, "tfidf")?;

            if !self.processed_files.contains_key(&chunk_.file) {
                // Recover B
                remove_file(&chunk_file)?;

                if exists(&tfidf_file) {
                    remove_file(&tfidf_file)?;
                }

                result.removed_chunk += 1;
                continue;
            }

            if exists(&tfidf_file) {
                if tfidf::load_from_file(&tfidf_file).is_err() {
                    chunk::save_to_file(
                        &chunk_file,
                        &chunk_,
                        self.build_config.compression_threshold,
                        self.build_config.compression_level,
                        &self.root_dir,
                    )?;
                }
            }

            match processed_files.get_mut(&chunk_.file) {
                Some(chunks) => {
                    chunks.push((chunk_.uid, chunk_.index));
                },
                None => {
                    processed_files.insert(chunk_.file.clone(), vec![(chunk_.uid, chunk_.index)]);
                },
            }

            chunk_count += 1;
        }

        // Recover A
        for dir in read_dir(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME,
            &FILE_INDEX_DIR_NAME,
        )?)? {
            remove_dir_all(&dir)?;
        }

        for (file, mut chunks) in processed_files.into_iter() {
            chunks.sort_by_key(|(_, index)| *index);
            let file_uid = self.processed_files.get(&file).unwrap();
            let file_index_path = Index::get_file_index_path(&self.root_dir, *file_uid);
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
            self.staged_files.push(curr_processing_file.clone());
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
                WriteMode::CreateOrTruncate,
            )?;
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
        }

        Ok(result)
    }
}
