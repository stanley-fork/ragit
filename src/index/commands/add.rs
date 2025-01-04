use super::Index;
use crate::INDEX_DIR_NAME;
use crate::error::Error;
use crate::uid::Uid;
use ragit_fs::{exists, is_dir, join, read_dir};
use std::fmt;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum AddMode {
    Force,
    Reject,
}

impl AddMode {
    pub fn parse_flag(flag: &str) -> Option<Self> {
        match flag {
            "--force" => Some(AddMode::Force),
            "--reject" => Some(AddMode::Reject),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct AddResult {
    staged: usize,
    ignored: usize,
}

impl fmt::Display for AddResult {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "{} files staged, {} files ignored", self.staged, self.ignored)
    }
}

impl Index {
    /// rag add
    /// |           | processed/modified | processed/not-modified |    staged    |    new    |  n exist  |
    /// |-----------|--------------------|------------------------|--------------|-----------|-----------|
    /// | ignore    | ignore             | ignore                 | ignore       | ignore    | error     |
    /// | n ignore  | stage              | ignore                 | ignore       | stage     | error     |
    ///
    /// rag add --reject
    /// |           | processed/modified | processed/not-modified |    staged    |    new    |  n exist  |
    /// |-----------|--------------------|------------------------|--------------|-----------|-----------|
    /// | ignore    | error              | error                  | error        | error     | error     |
    /// | n ignore  | error              | error                  | error        | stage     | error     |
    ///
    /// rag add --force
    /// |           | processed/modified | processed/not-modified |    staged    |    new    |  n exist  |
    /// |-----------|--------------------|------------------------|--------------|-----------|-----------|
    /// | ignore    | stage              | ignore                 | ignore       | stage     | error     |
    /// | n ignore  | stage              | ignore                 | ignore       | stage     | error     |
    pub fn add_files(
        &mut self,
        files: &Vec<String>,
        mode: Option<AddMode>,
        dry_run: bool,
        ignore: &Ignore,
    ) -> Result<AddResult, Error> {
        let mut result = AddResult::default();

        if files.is_empty() {
            return Ok(result);
        }

        if let Some(file) = &self.curr_processing_file {
            return Err(Error::CannotAddFile {
                file: Index::get_rel_path(&self.root_dir, &files[0])?,
                message: format!("A build process has been interrupted while processing `{file}`. Please clean it up."),
            });
        }

        let mut unfolded_files = vec![];

        for file in files.iter() {
            if !exists(file) {
                return Err(Error::CannotAddFile {
                    file: Index::get_rel_path(&self.root_dir, file)?,
                    message: format!("`{file}` does not exist."),
                });
            }

            else if is_dir(file) {
                for (ignored, sub) in walk_tree_with_ignore_file(file, ignore)? {
                    if ignored {
                        match mode {
                            None => {
                                result.ignored += 1;
                                continue;
                            },
                            Some(AddMode::Reject) => {
                                return Err(Error::CannotAddFile {
                                    file: Index::get_rel_path(&self.root_dir, &sub)?,
                                    message: format!("`{sub}` is ignored."),
                                });
                            },
                            Some(AddMode::Force) => {},
                        }
                    }

                    unfolded_files.push(Index::get_rel_path(&self.root_dir, file)?);
                }
            }

            else if has_to_be_ignored(file, ignore)? {
                match mode {
                    None => {
                        result.ignored += 1;
                    },
                    Some(AddMode::Reject) => {
                        return Err(Error::CannotAddFile {
                            file: Index::get_rel_path(&self.root_dir, file)?,
                            message: format!("`{file}` is ignored."),
                        });
                    },
                    Some(AddMode::Force) => {
                        unfolded_files.push(Index::get_rel_path(&self.root_dir, file)?);
                    },
                }
            }

            else {
                unfolded_files.push(Index::get_rel_path(&self.root_dir, file)?);
            }
        }

        let mut newly_staged_files = vec![];
        let mut ignored_file: Option<String> = None;  // for an error message

        for file in unfolded_files.iter() {
            // NOTE: ragit never allows you to add files in `.ragit/`
            // FIXME: `.starts_with` is for strings, not for paths. It works for most cases, but not always
            if file.starts_with(INDEX_DIR_NAME) {
                result.ignored += 1;
                ignored_file = Some(file.to_string());
            }

            else if self.staged_files.contains(file) {
                result.ignored += 1;
                ignored_file = Some(file.to_string());
            }

            else if let Some(prev_hash) = self.processed_files.get(file) {
                let curr_hash = Uid::new_file(&self.root_dir, &join(&self.root_dir, &file)?)?;

                match (mode, *prev_hash != curr_hash) {
                    (None, true) => {
                        result.staged += 1;
                        newly_staged_files.push(file.to_string());
                    },
                    (None, false) => {
                        result.ignored += 1;
                        ignored_file = Some(file.to_string());
                    },
                    (Some(AddMode::Reject), _) => {
                        return Err(Error::CannotAddFile {
                            file: file.to_string(),
                            message: format!("`{file}` is already processed."),
                        });
                    },
                    (Some(AddMode::Force), true) => {
                        result.staged += 1;
                        newly_staged_files.push(file.to_string());
                    },
                    (Some(AddMode::Force), false) => {
                        result.ignored += 1;
                        ignored_file = Some(file.to_string());
                    },
                }
            }

            else {
                result.staged += 1;
                newly_staged_files.push(file.to_string());
            }
        }

        if result.ignored > 0 && mode == Some(AddMode::Reject) {
            let ignored_file = ignored_file.unwrap();

            return Err(Error::CannotAddFile {
                file: ignored_file.clone(),
                message: format!("`{ignored_file}` is ignored."),
            });
        }

        if !dry_run {
            self.staged_files.extend(newly_staged_files);
        }

        Ok(result)
    }

    pub fn read_ignore_file(&self) -> Result<Ignore, Error> {
        Ok(Ignore {})  // TODO
    }
}

// TODO
pub struct Ignore {}

fn walk_tree_with_ignore_file(
    dir: &str,
    ignore: &Ignore,
) -> Result<Vec<(bool, String)>, Error> {
    let mut result = vec![];
    walk_tree_with_ignore_file_worker(dir, ignore, &mut result)?;
    Ok(result)
}

fn walk_tree_with_ignore_file_worker(
    file: &str,
    ignore: &Ignore,
    buffer: &mut Vec<(bool, String)>,
) -> Result<(), Error> {
    if is_dir(file) {
        for entry in read_dir(file, false)? {
            walk_tree_with_ignore_file_worker(&entry, ignore, buffer)?;
        }
    }

    else {
        buffer.push((has_to_be_ignored(file, ignore)?, file.to_string()));
    }

    Ok(())
}

fn has_to_be_ignored(file: &str, ignore: &Ignore) -> Result<bool, Error> {
    Ok(false)  // TODO
}
