use super::Index;
use crate::INDEX_DIR_NAME;
use crate::error::Error;
use crate::index::get_file_hash;

/// "force update" means `rag remove FILE` + `rag add FILE`
#[derive(Copy, Clone)]
pub enum AddMode {
    /// If the file is already in the index, it force updates file.
    Force,

    /// if the file is already in the index, it first checks whether the file has been modified
    /// the file is force updated only if modified
    Auto,

    /// if the files is already in the index, nothing happens
    Ignore,
}

impl AddMode {
    pub fn parse_flag(flag: &str) -> Option<Self> {
        match flag {
            "--force" => Some(AddMode::Force),
            "--auto" => Some(AddMode::Auto),
            "--ignore" => Some(AddMode::Ignore),
            _ => None,
        }
    }
}

pub enum AddResult {
    Added,
    Ignored,
    Updated,
}

impl Index {
    /// It adds a file to the staging area.
    /// This function is what runs the `rag add` command.
    pub fn add_file(
        &mut self,
        path: &str,  // real_path
        mode: AddMode,
    ) -> Result<AddResult, Error> {
        let rel_path = Index::get_rel_path(&self.root_dir, &path.to_string());

        // you cannot add a file that's inside `.rag_index`
        if rel_path.starts_with(INDEX_DIR_NAME) {  // TODO: `starts_with` is for strings, not for paths
            return Ok(AddResult::Ignored);
        }

        match mode {
            AddMode::Force => {
                if self.staged_files.contains(&rel_path) {
                    // if it's not processed yet, `rag remove FILE` + `rag add FILE` is nop
                    return Ok(AddResult::Updated);
                }

                else if self.processed_files.contains_key(&rel_path) || self.curr_processing_file == Some(rel_path.to_string()) {
                    self.remove_file(path.to_string())?;
                    self.staged_files.push(rel_path);
                    return Ok(AddResult::Updated);
                }
            },
            AddMode::Auto => {
                if self.staged_files.contains(&rel_path) {
                    // if it's not processed yet, it cannot compare the contents
                    return Ok(AddResult::Ignored);
                }

                else if let Some(prev_hash) = self.processed_files.get(&rel_path) {
                    if let Ok(hash) = get_file_hash(&path.to_string()) {
                        if &hash != prev_hash {
                            self.remove_file(path.to_string())?;
                            self.staged_files.push(rel_path);
                            return Ok(AddResult::Updated);
                        }

                        else {
                            return Ok(AddResult::Ignored);
                        }
                    }

                    else {  // the file is deleted
                        // 1. deletion is also a modification
                        // 2. adding a non-existent file is not an error
                        self.remove_file(path.to_string())?;
                        self.staged_files.push(rel_path);
                        return Ok(AddResult::Updated);
                    }
                }

                else if self.curr_processing_file == Some(path.to_string()) {
                    self.remove_file(path.to_string())?;
                    self.staged_files.push(rel_path);
                    return Ok(AddResult::Updated);
                }
            },
            AddMode::Ignore => {
                if self.staged_files.contains(&rel_path) || self.processed_files.contains_key(&rel_path) || self.curr_processing_file == Some(rel_path.to_string()) {
                    return Ok(AddResult::Ignored);
                }
            },
        }

        self.staged_files.push(rel_path);
        Ok(AddResult::Added)
    }
}
