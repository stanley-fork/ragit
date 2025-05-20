use super::Index;
use crate::constant::INDEX_DIR_NAME;
use crate::error::Error;
use crate::uid::Uid;
use ragit_fs::{exists, get_relative_path, is_dir, is_symlink, join, read_string};
use ragit_ignore::Ignore;
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
        let force = mode == Some(AddMode::Force);

        if self.curr_processing_file.is_some() {
            return Err(Error::DirtyKnowledgeBase);
        }

        if files.is_empty() {
            return Ok(result);
        }

        let mut unfolded_files = vec![];

        for file in files.iter() {
            let relative_path = get_relative_path(&self.root_dir, file)?;

            // `.starts_with` would work because `relative_path` is normalized.
            // But still, it's too ugly.
            if relative_path.starts_with("../") {
                return Err(Error::CannotAddFile {
                    file: relative_path,
                    message: format!("`{file}` is outside of knowledge-base."),
                });
            }

            if !exists(file) {
                return Err(Error::CannotAddFile {
                    file: relative_path,
                    message: format!("`{file}` does not exist."),
                });
            }

            // it filters out `.ragit/` and `.git/`
            else if ignore.is_strong_match(&self.root_dir, file) {
                continue;
            }

            else if is_symlink(file) {
                result.ignored += 1;
            }

            else if is_dir(file) {
                for (ignored, sub) in ignore.walk_tree(&self.root_dir, file, false /* follow symlink */, !force /* skip ignored dirs */)? {
                    if ignored {
                        match mode {
                            None => {
                                result.ignored += 1;
                                continue;
                            },
                            Some(AddMode::Reject) => {
                                return Err(Error::CannotAddFile {
                                    file: get_relative_path(&self.root_dir, &sub)?,
                                    message: format!("`{sub}` is ignored."),
                                });
                            },
                            Some(AddMode::Force) => {},
                        }
                    }

                    unfolded_files.push(get_relative_path(&self.root_dir, &sub)?);
                }
            }

            else if ignore.is_match(&self.root_dir, file) {
                match mode {
                    None => {
                        result.ignored += 1;
                    },
                    Some(AddMode::Reject) => {
                        return Err(Error::CannotAddFile {
                            file: relative_path,
                            message: format!("`{file}` is ignored."),
                        });
                    },
                    Some(AddMode::Force) => {
                        unfolded_files.push(relative_path);
                    },
                }
            }

            else {
                unfolded_files.push(get_relative_path(&self.root_dir, file)?);
            }
        }

        let mut newly_staged_files = vec![];
        let mut ignored_file: Option<String> = None;  // for an error message

        for file in unfolded_files.iter() {
            if self.staged_files.contains(file) {
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

        self.reset_uid(false /* save to file */)?;

        if !dry_run {
            self.staged_files.extend(newly_staged_files);
            self.save_to_file()?;
        }

        Ok(result)
    }

    pub fn read_ignore_file(&self) -> Result<Ignore, Error> {
        let mut ignore_file_at = String::new();

        for ignore_file in [
            ".ragignore",
            ".gitignore",
            ".ignore",
        ] {
            ignore_file_at = join(
                &self.root_dir,
                ignore_file,
            )?;

            if exists(&ignore_file_at) {
                break;
            }
        }

        let mut result = if !exists(&ignore_file_at) {
            Ignore::new()
        }

        else {
            Ignore::parse(&read_string(&ignore_file_at)?)
        };

        result.add_strong_pattern(".git");
        result.add_strong_pattern(INDEX_DIR_NAME);
        result.add_strong_pattern(".ragignore");
        // result.add_strong_pattern(".gitignore");  -> it's tracked by git!

        Ok(result)
    }
}
