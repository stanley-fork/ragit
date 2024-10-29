use super::Index;
use crate::chunk;
use crate::error::Error;
use crate::index::{CHUNK_INDEX_DIR_NAME, INDEX_DIR_NAME};
use ragit_fs::{file_name, join, read_dir, remove_file};
use std::collections::{HashMap, HashSet};

impl Index {
    pub fn auto_recover(&mut self) -> Result<(), Error> {
        let curr_processing_file = self.curr_processing_file.clone();
        self.curr_processing_file = None;
        let mut chunk_files_to_remove = vec![];

        // It's re-created from scratch
        let mut chunk_index_map = HashMap::new();

        // It removes unused images
        let mut images = HashSet::new();

        for chunk_file in self.chunk_files_real_path() {
            match chunk::load_from_file(&chunk_file) {
                Ok(chunks) => {
                    let mut new_chunks = Vec::with_capacity(chunks.len());

                    for chunk in chunks.into_iter() {
                        if let Some(file) = &curr_processing_file {
                            if &chunk.file == file {
                                continue;
                            }
                        }

                        chunk_index_map.insert(chunk.uid.clone(), file_name(&chunk_file)?);

                        for image in chunk.images.iter() {
                            images.insert(image.to_string());
                        }

                        new_chunks.push(chunk);
                    }

                    // It also re-creates tfidf indexes
                    chunk::save_to_file(&chunk_file, &new_chunks, self.config.compression_threshold, self.config.compression_level)?;
                },
                Err(_) => {
                    chunk_files_to_remove.push(chunk_file);
                },
            }
        }

        // TODO: remove all chunk_index files
        for chunk_index_file in read_dir(&join(
            &self.root_dir,
            &join(
                &INDEX_DIR_NAME.to_string(),
                &CHUNK_INDEX_DIR_NAME.to_string(),
            )?,
        )?)? {
            remove_file(&chunk_index_file)?;
        }

        for (chunk_uid, chunk_index) in chunk_index_map.iter() {
            self.add_chunk_index(chunk_uid, chunk_index)?;
        }

        // TODO: remove unused images

        Ok(())
    }
}
