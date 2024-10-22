use super::Index;
use crate::INDEX_DIR_NAME;
use crate::error::Error;
use crate::index::get_file_hash;

// "force update" means `rag remove FILE` + `rag add FILE`
#[derive(Copy, Clone)]
pub enum AddMode {
    // if the file is already in the index, it force updates file
    Force,

    // if the file is already in the index, it first checks whether the file has been modified
    // the file is force updated only if modified
    Auto,

    // if the files is already in the index, nothing happens
    Ignore,
}

impl From<&String> for AddMode {
    fn from(s: &String) -> Self {
        match s {
            s if s == "--force" => AddMode::Force,
            s if s == "--auto" => AddMode::Auto,
            s if s == "--ignore" => AddMode::Ignore,
            s => panic!("invalid flag for `add` command: {s:?}"),
        }
    }
}

pub enum AddResult {
    Added,
    Ignored,
    Updated,
}

impl Index {
    pub fn add_file(
        &mut self,
        path: String,  // normalized rel_path
        mode: AddMode,
    ) -> Result<AddResult, Error> {
        // you cannot add a file that's inside `.rag_index`
        if path.starts_with(INDEX_DIR_NAME) {  // TODO: `starts_with` is for strings, not for paths
            return Ok(AddResult::Ignored);
        }

        match mode {
            AddMode::Force => {
                if self.staged_files.contains(&path) {
                    // if it's not processed yet, `rag remove FILE` + `rag add FILE` is nop
                    return Ok(AddResult::Updated);
                }

                else if self.processed_files.contains_key(&path) || self.curr_processing_file == Some(path.to_string()) {
                    self.remove_file(path.clone())?;
                    self.staged_files.push(path);
                    return Ok(AddResult::Updated);
                }
            },
            AddMode::Auto => {
                if self.staged_files.contains(&path) {
                    // if it's not processed yet, it cannot compare the contents
                    return Ok(AddResult::Ignored);
                }

                else if let Some(prev_hash) = self.processed_files.get(&path) {
                    let real_path = Index::get_data_path(
                        &self.root_dir,
                        &path,
                    );

                    if let Ok(hash) = get_file_hash(&real_path) {
                        if &hash != prev_hash {
                            self.remove_file(path.clone())?;
                            self.staged_files.push(path);
                            return Ok(AddResult::Updated);
                        }

                        else {
                            return Ok(AddResult::Ignored);
                        }
                    }

                    else {  // the file is deleted
                        // 1. deletion is also a modification
                        // 2. adding a non-existent file is not an error
                        self.remove_file(path.clone())?;
                        self.staged_files.push(path);
                        return Ok(AddResult::Updated);
                    }
                }

                else if self.curr_processing_file == Some(path.to_string()) {
                    self.remove_file(path.clone())?;
                    self.staged_files.push(path);
                    return Ok(AddResult::Updated);
                }
            },
            AddMode::Ignore => {
                if self.staged_files.contains(&path) || self.processed_files.contains_key(&path) || self.curr_processing_file == Some(path.to_string()) {
                    return Ok(AddResult::Ignored);
                }
            },
        }

        self.staged_files.push(path);
        Ok(AddResult::Added)
    }
}
