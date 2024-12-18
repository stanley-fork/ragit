use super::Index;
use crate::INDEX_DIR_NAME;
use crate::error::Error;
use crate::uid::Uid;

/// "force update" means `rag remove FILE` + `rag add FILE`
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum AddMode {
    /// If the file is already in the index, it force updates file.
    Force,

    /// If the file is already in the index, it first checks whether the file has been modified
    /// the file is force updated only if modified
    Auto,

    /// If the files is already in the index, nothing happens
    Ignore,

    /// If the file is already in the index, it does nothing and returns an error.
    Reject,
}

impl AddMode {
    pub fn parse_flag(flag: &str) -> Option<Self> {
        match flag {
            "--force" => Some(AddMode::Force),
            "--auto" => Some(AddMode::Auto),
            "--ignore" => Some(AddMode::Ignore),
            "--reject" => Some(AddMode::Reject),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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
        dry_run: bool,
    ) -> Result<AddResult, Error> {
        let rel_path = Index::get_rel_path(&self.root_dir, &path.to_string())?;

        // you cannot add a file that's inside `.ragit/`
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
                    if !dry_run {
                        self.remove_file(path.to_string())?;
                        self.staged_files.push(rel_path);
                    }

                    return Ok(AddResult::Updated);
                }
            },
            AddMode::Auto => {
                if self.staged_files.contains(&rel_path) {
                    // if it's not processed yet, it cannot compare the contents
                    return Ok(AddResult::Ignored);
                }

                else if let Some(prev_hash) = self.processed_files.get(&rel_path) {
                    if let Ok(hash) = Uid::new_file(&self.root_dir, path) {
                        if &hash != prev_hash {
                            if !dry_run {
                                self.remove_file(path.to_string())?;
                                self.staged_files.push(rel_path);
                            }

                            return Ok(AddResult::Updated);
                        }

                        else {
                            return Ok(AddResult::Ignored);
                        }
                    }

                    else {  // the file is deleted
                        // 1. deletion is also a modification
                        // 2. adding a non-existent file is not an error
                        if !dry_run {
                            self.remove_file(path.to_string())?;
                            self.staged_files.push(rel_path);
                        }

                        return Ok(AddResult::Updated);
                    }
                }

                else if self.curr_processing_file == Some(path.to_string()) {
                    if !dry_run {
                        self.remove_file(path.to_string())?;
                        self.staged_files.push(rel_path);
                    }

                    return Ok(AddResult::Updated);
                }
            },
            AddMode::Ignore => {
                if self.staged_files.contains(&rel_path) || self.processed_files.contains_key(&rel_path) || self.curr_processing_file == Some(rel_path.to_string()) {
                    return Ok(AddResult::Ignored);
                }
            },
            AddMode::Reject => {
                if self.staged_files.contains(&rel_path) || self.processed_files.contains_key(&rel_path) || self.curr_processing_file == Some(rel_path.to_string()) {
                    return Err(Error::AddConflict(rel_path.to_string()));
                }
            },
        }

        if !dry_run {
            self.staged_files.push(rel_path);
        }

        Ok(AddResult::Added)
    }
}
