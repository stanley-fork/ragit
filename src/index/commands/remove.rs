use super::Index;
use crate::error::Error;
use crate::index::{CHUNK_DIR_NAME, IIStatus};
use ragit_fs::{exists, get_relative_path, remove_file, set_extension};

pub type Path = String;

impl Index {
    pub fn remove_file(
        &mut self,
        path: Path,  // real_path
    ) -> Result<(), Error> {
        let rel_path = get_relative_path(&self.root_dir, &path)?;

        if self.staged_files.contains(&rel_path) {
            self.staged_files = self.staged_files.iter().filter(
                |file| file.to_string() != rel_path
            ).map(
                |file| file.to_string()
            ).collect();

            Ok(())
        }

        else if self.processed_files.contains_key(&rel_path) || self.curr_processing_file == Some(rel_path.clone()) {
            self.ii_status = IIStatus::Outdated;

            match self.processed_files.get(&rel_path).map(|uid| *uid) {
                Some(file_uid) => {
                    for uid in self.get_chunks_of_file(file_uid)? {
                        self.chunk_count -= 1;
                        let chunk_path = Index::get_uid_path(
                            &self.root_dir,
                            CHUNK_DIR_NAME,
                            uid,
                            Some("chunk"),
                        )?;
                        remove_file(&chunk_path)?;
                        let tfidf_path = set_extension(&chunk_path, "tfidf")?;

                        if exists(&tfidf_path) {
                            remove_file(&tfidf_path)?;
                        }
                    }

                    self.processed_files.remove(&rel_path).unwrap();
                    self.remove_file_index(file_uid)?;
                },
                None => {
                    self.curr_processing_file = None;
                },
            }

            Ok(())
        }

        else {
            Err(Error::NoSuchFile { path: Some(path), uid: None })
        }
    }

    pub fn remove_auto(&mut self) -> Result<Vec<Path>, Error> {  // it returns a list of removed files
        let mut files_to_remove = vec![];

        for staged_file in self.staged_files.iter() {
            if !exists(&Index::get_data_path(&self.root_dir, staged_file)?) {
                files_to_remove.push(staged_file.to_string());
            }
        }

        for processed_file in self.processed_files.keys() {
            if !exists(&Index::get_data_path(&self.root_dir, processed_file)?) {
                files_to_remove.push(processed_file.to_string());
            }
        }

        if let Some(file) = &self.curr_processing_file {
            if !exists(&Index::get_data_path(&self.root_dir, file)?) {
                files_to_remove.push(file.to_string());
            }
        }

        files_to_remove = files_to_remove.into_iter().map(|file| Index::get_data_path(&self.root_dir, &file).unwrap()).collect();

        for file in files_to_remove.iter() {
            self.remove_file(file.clone())?;
        }

        Ok(files_to_remove)
    }
}
