use super::Index;
use crate::error::Error;
use crate::index::LOG_DIR_NAME;
use ragit_fs::{
    read_dir,
    remove_file,
};

impl Index {
    pub fn gc_logs(&self) -> Result<(), Error> {
        let logs_at = Index::get_rag_path(
            &self.root_dir,
            &LOG_DIR_NAME.to_string(),
        );

        for file in read_dir(&logs_at)? {
            remove_file(&file)?;
        }

        Ok(())
    }
}
