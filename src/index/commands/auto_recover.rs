use super::Index;
use crate::{ApiConfigRaw, BuildConfig, QueryConfig, chunk};
use crate::error::Error;
use crate::index::{
    CHUNK_DIR_NAME,
    CHUNK_INDEX_DIR_NAME,
    IMAGE_DIR_NAME,
    INDEX_DIR_NAME,
    UpdateTfidf,
};
use json::JsonValue;
use ragit_fs::{
    WriteMode,
    exists,
    file_name,
    join3,
    read_dir,
    read_string,
    remove_file,
    set_extension,
    write_bytes,
};
use std::collections::{HashMap, HashSet};

impl Index {
    /// This is `auto-recover` of `rag check --auto-recover`. It tries its best to make the index usable.
    /// It may remove some chunks if necessary information is missing.
    ///
    /// - Recover A: If `self.curr_processing_file` exists, remove all the chunks related to it and add the file to the staging area.
    ///   - `self.curr_processing_file` exists if previous `rag build` was interrupted.
    /// - Recover B: Create chunk_index files from scratch by reading the actual chunk files.
    /// - Recover C: Replace config json files with their default values if broken.
    /// - Recover D: Count `self.chunk_count`.
    /// - Recover E: If chunks are missing, it updates `self.processed_files`.
    ///   - It can remove entries in `self.processed_files`, but cannot add ones.
    ///   - In order to add an entry, it needs an actual file, which are missing if the knowledge-base was cloned from remote.
    pub fn auto_recover(&mut self) -> Result<(), Error> {
        let curr_processing_file = self.curr_processing_file.clone();
        self.curr_processing_file = None;  // Recover A
        let mut chunk_files_to_remove = vec![];

        // It's re-created from scratch
        let mut chunk_index_map = HashMap::new();
        let mut chunk_files = HashMap::new();
        let mut chunk_count = 0;
        let mut processed_files = HashSet::new();

        // It removes unused images
        let mut images = HashSet::new();

        for chunk_file in read_dir(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &CHUNK_DIR_NAME.to_string(),
        )?)? {
            match chunk::load_from_file(&chunk_file) {
                Ok(chunks) => {
                    let mut new_chunks = Vec::with_capacity(chunks.len());
                    let chunk_file_name = file_name(&chunk_file)?;

                    if chunks.is_empty() {
                        chunk_files.insert(chunk_file_name.clone(), 0);
                    }

                    for chunk in chunks.into_iter() {
                        if let Some(file) = &curr_processing_file {
                            if &chunk.file == file {
                                continue;
                            }
                        }

                        chunk_index_map.insert(chunk.uid.clone(), chunk_file_name.clone());

                        match chunk_files.get_mut(&chunk_file_name) {
                            Some(n) => { *n += 1; },
                            None => { chunk_files.insert(chunk_file_name.clone(), 1); },
                        }

                        for image in chunk.images.iter() {
                            images.insert(image.to_string());
                        }

                        processed_files.insert(chunk.file.clone());
                        new_chunks.push(chunk);
                        chunk_count += 1;
                    }

                    // It also re-creates tfidf indexes
                    chunk::save_to_file(
                        &chunk_file,
                        &new_chunks,
                        self.build_config.compression_threshold,
                        self.build_config.compression_level,
                        &self.root_dir,
                        UpdateTfidf::Generate,
                    )?;
                },
                Err(_) => {
                    chunk_files_to_remove.push(chunk_file);
                },
            }
        }

        for chunk_index_file in read_dir(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &CHUNK_INDEX_DIR_NAME.to_string(),
        )?)? {
            remove_file(&chunk_index_file)?;
        }

        // Recover B
        for (chunk_uid, chunk_index) in chunk_index_map.iter() {
            self.add_chunk_index(chunk_uid, chunk_index, false)?;
        }

        let mut images_to_remove = vec![];

        for image_file in read_dir(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &IMAGE_DIR_NAME.to_string(),
        )?)? {
            // 1. At least one chunk has this image.
            if !images.contains(&file_name(&image_file)?) {
                images_to_remove.push(image_file);
                continue;
            }

            // 2. Its description file is a valid json object.
            match read_string(&set_extension(&image_file, "json")?) {
                Ok(j) => match json::parse(&j) {
                    Ok(JsonValue::Object(_)) => {},
                    _ => {
                        images_to_remove.push(image_file);
                        continue;
                    },
                },
                Err(_) => {
                    images_to_remove.push(image_file);
                    continue;
                },
            }

            // 3. Both png file and json file exist.
            if !exists(&set_extension(&image_file, "png")?) || !exists(&set_extension(&image_file, "json")?) {
                images_to_remove.push(image_file);
            }
        }

        for image_file in images_to_remove {
            remove_file(&set_extension(&image_file, "png")?)?;
            remove_file(&set_extension(&image_file, "json")?)?;
        }

        // Recover C
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

        // Recover D
        self.chunk_files = chunk_files;
        self.chunk_count = chunk_count;

        // Recover A
        if let Some(curr_processing_file) = curr_processing_file {
            self.staged_files.push(curr_processing_file);
        }

        // Recover E
        for processed_file in processed_files.iter() {
            // It cannot add a new file to `self.processed_files`. See the comments above.
            if !self.processed_files.contains_key(processed_file) {
                return Err(Error::BrokenIndex(format!("!self.processed_files.contains_key({processed_file:?})")));
            }
        }

        let mut files_to_remove = vec![];

        for processed_file in self.processed_files.keys() {
            if !processed_files.contains(processed_file) {
                files_to_remove.push(processed_file.to_string());
            }
        }

        for file in files_to_remove.iter() {
            self.processed_files.remove(file);

            if !self.staged_files.contains(file) {
                self.staged_files.push(file.to_string());
            }
        }

        Ok(())
    }
}
