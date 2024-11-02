use super::Index;
use crate::error::Error;
use crate::index::LOG_DIR_NAME;
use ragit_fs::{
    read_dir,
    remove_file,
};

impl Index {
    /// `rag gc --logs`
    pub fn gc_logs(&self) -> Result<usize, Error> {  // returns how many files it removed
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

    // TODO: `gc_images`
}
