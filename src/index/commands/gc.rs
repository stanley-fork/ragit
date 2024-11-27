use super::Index;
use crate::INDEX_DIR_NAME;
use crate::chunk;
use crate::error::Error;
use crate::index::{IMAGE_DIR_NAME, LOG_DIR_NAME};
use crate::uid::Uid;
use ragit_fs::{
    extension,
    file_name,
    join3,
    read_dir,
    remove_file,
};
use std::collections::HashSet;

impl Index {
    /// `rag gc --logs`
    ///
    /// It returns how many files it removed.
    pub fn gc_logs(&self) -> Result<usize, Error> {
        let logs_at = Index::get_rag_path(
            &self.root_dir,
            &LOG_DIR_NAME.to_string(),
        );
        let mut count = 0;

        for file in read_dir(&logs_at)? {
            count += 1;
            remove_file(&file)?;
        }

        Ok(count)
    }

    /// `rag gc --images`
    ///
    /// It returns how many files it removed.
    pub fn gc_images(&self) -> Result<usize, Error> {
        let mut all_images = HashSet::new();
        let mut count = 0;

        for chunk_file in self.get_all_chunk_files()? {
            for image in chunk::load_from_file(&chunk_file)?.images {
                all_images.insert(image);
            }
        }

        for file in read_dir(&join3(
            &self.root_dir,
            INDEX_DIR_NAME,
            IMAGE_DIR_NAME,
        )?)? {
            let uid = file_name(&file)?.parse::<Uid>()?;

            if !all_images.contains(&uid) {
                remove_file(&file)?;
                count += 1;
            }
        }

        Ok(count)
    }
}
