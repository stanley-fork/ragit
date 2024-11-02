use super::Index;
use crate::chunk;
use crate::error::Error;
use crate::index::{CHUNK_DIR_NAME, CHUNK_INDEX_DIR_NAME, IMAGE_DIR_NAME, INDEX_DIR_NAME};
use ragit_fs::{file_name, join3, read_dir, remove_file};
use std::collections::{HashMap, HashSet};

impl Index {
    /// This is `auto-recover` of `rag check --auto-recover`.
    pub fn auto_recover(&mut self) -> Result<(), Error> {
        let curr_processing_file = self.curr_processing_file.clone();
        self.curr_processing_file = None;
        let mut chunk_files_to_remove = vec![];

        // It's re-created from scratch
        let mut chunk_index_map = HashMap::new();
        let mut chunk_files = HashMap::new();
        let mut chunk_count = 0;

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

                        new_chunks.push(chunk);
                        chunk_count += 1;
                    }

                    // It also re-creates tfidf indexes
                    chunk::save_to_file(&chunk_file, &new_chunks, self.config.compression_threshold, self.config.compression_level)?;
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

        for (chunk_uid, chunk_index) in chunk_index_map.iter() {
            self.add_chunk_index(chunk_uid, chunk_index, false)?;
        }

        for image_file in read_dir(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &IMAGE_DIR_NAME.to_string(),
        )?)? {
            if !images.contains(&file_name(&image_file)?) {
                remove_file(&image_file)?;
            }
        }

        self.chunk_files = chunk_files;
        self.chunk_count = chunk_count;

        if self.chunk_count == 0 {
            self.create_new_chunk_file()?;
        }

        if let Some(curr_processing_file) = curr_processing_file {
            self.staged_files.push(curr_processing_file);
        }

        Ok(())
    }
}
