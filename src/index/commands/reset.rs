use super::Index;
use crate::error::Error;
use crate::index::{CHUNK_DIR_NAME, FILE_INDEX_DIR_NAME, IMAGE_DIR_NAME, INDEX_DIR_NAME};
use ragit_fs::{create_dir_all, join, join3, remove_dir_all};
use std::collections::HashMap;

impl Index {
    pub fn reset_hard(root_dir: &str) -> Result<(), Error> {
        remove_dir_all(&join(
            &root_dir,
            &INDEX_DIR_NAME.to_string(),
        )?)?;

        Ok(())
    }

    pub fn reset_soft(&mut self) -> Result<(), Error> {
        self.chunk_count = 0;
        self.staged_files = vec![];
        self.processed_files = HashMap::new();
        self.curr_processing_file = None;

        remove_dir_all(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &CHUNK_DIR_NAME.to_string(),
        )?)?;
        create_dir_all(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &CHUNK_DIR_NAME.to_string(),
        )?)?;
        remove_dir_all(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &IMAGE_DIR_NAME.to_string(),
        )?)?;
        create_dir_all(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &IMAGE_DIR_NAME.to_string(),
        )?)?;
        remove_dir_all(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &FILE_INDEX_DIR_NAME.to_string(),
        )?)?;
        create_dir_all(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME.to_string(),
            &FILE_INDEX_DIR_NAME.to_string(),
        )?)?;

        Ok(())
    }
}
