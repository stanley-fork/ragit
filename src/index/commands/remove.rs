use super::Index;
use crate::error::Error;
use ragit_fs::exists;

pub type Path = String;

impl Index {
    pub fn remove_file(
        &mut self,
        path: Path,  // normalized rel_path
    ) -> Result<(), Error> {
        if self.staged_files.contains(&path) {
            self.staged_files = self.staged_files.iter().filter(
                |file| file.to_string() != path
            ).map(
                |file| file.to_string()
            ).collect();

            Ok(())
        }

        else if self.processed_files.contains_key(&path) || self.curr_processing_file == Some(path.clone()) {
            self.remove_chunks_by_file_name(path.clone())?;

            if self.curr_processing_file == Some(path.clone()) {
                self.curr_processing_file = None;
            }

            else {
                self.processed_files.remove(&path).unwrap();
            }

            Ok(())
        }

        else {
            Err(Error::NoSuchFile { file: path })
        }
    }

    pub fn remove_auto(&mut self) -> Result<Vec<Path>, Error> {  // it returns a list of removed files
        let mut files_to_remove = vec![];

        for staged_file in self.staged_files.iter() {
            if !exists(&Index::get_data_path(&self.root_dir, staged_file)) {
                files_to_remove.push(staged_file.to_string());
            }
        }

        for processed_file in self.processed_files.keys() {
            if !exists(&Index::get_data_path(&self.root_dir, processed_file)) {
                files_to_remove.push(processed_file.to_string());
            }
        }

        if let Some(file) = &self.curr_processing_file {
            if !exists(&Index::get_data_path(&self.root_dir, file)) {
                files_to_remove.push(file.to_string());
            }
        }

        for file in files_to_remove.iter() {
            self.remove_file(file.clone())?;
        }

        Ok(files_to_remove)
    }
}
