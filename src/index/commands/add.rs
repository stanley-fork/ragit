use super::Index;
use crate::INDEX_DIR_NAME;
use crate::error::Error;
use crate::uid::Uid;
use ragit_fs::{exists, is_dir, join, read_dir, read_string};
use regex::Regex;
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
                for (ignored, sub) in ignore.walk_tree(&self.root_dir, file)? {
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

                    unfolded_files.push(Index::get_rel_path(&self.root_dir, &sub)?);
                }
            }

            else if ignore.is_match(&self.root_dir, file) {
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
            if is_implicitly_ignored_file(file) {
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
            self.save_to_file()?;
        }

        Ok(result)
    }

    pub fn read_ignore_file(&self) -> Result<Ignore, Error> {
        let ignore_file_at = join(
            &self.root_dir,
            ".ragignore",
        )?;

        if !exists(&ignore_file_at) {
            Ok(Ignore::new())
        }

        else {
            Ok(Ignore::parse(&read_string(&ignore_file_at)?))
        }
    }
}

// It should now allow users to add files in `.ragit/`, or any other file
// that's directly related to ragit
fn is_implicitly_ignored_file(rel_path: &str) -> bool {
    let splitted = rel_path.split("/").map(|s| s.to_string()).collect::<Vec<_>>();

    match splitted.first().map(|s| s.as_str()) {
        Some(ragit) if ragit == INDEX_DIR_NAME => true,
        Some(".ragignore") if splitted.len() == 1 => true,
        _ => false,
    }
}

pub struct Ignore {
    patterns: Vec<IgnorePattern>,
}

impl Ignore {
    pub fn new() -> Self {
        Ignore {
            patterns: vec![],
        }
    }

    // like `.gitignore`, `.ragignore` never fails to parse
    pub fn parse(s: &str) -> Self {
        let mut patterns = vec![];

        for line in s.lines() {
            let t = line.trim();

            if t.is_empty() || t.starts_with("#") {
                continue;
            }

            let mut t = t.to_string();

            if !t.starts_with("/") {
                t = format!("**/{t}");
            }

            patterns.push(IgnorePattern::parse(&t));
        }

        Ignore { patterns }
    }

    pub fn walk_tree(&self, root_dir: &str, dir: &str) -> Result<Vec<(bool, String)>, Error> {
        let mut result = vec![];
        self.walk_tree_worker(root_dir, dir, &mut result)?;
        Ok(result)
    }

    fn walk_tree_worker(&self, root_dir: &str, file: &str, buffer: &mut Vec<(bool, String)>) -> Result<(), Error> {
        if is_dir(file) {
            if self.is_match(root_dir, file) {
                buffer.push((true, file.to_string()));
            }

            else {
                for entry in read_dir(file, false)? {
                    self.walk_tree_worker(root_dir, &entry, buffer)?;
                }
            }
        }

        else {
            buffer.push((self.is_match(root_dir, file), file.to_string()));
        }

        Ok(())
    }

    pub fn is_match(&self, root_dir: &str, file: &str) -> bool {
        let Ok(rel_path) = Index::get_rel_path(&root_dir.to_string(), &file.to_string()) else { return false; };

        for pattern in self.patterns.iter() {
            if pattern.is_match(&rel_path) {
                return true;
            }
        }

        false
    }
}

pub struct IgnorePattern {
    // TODO: I need a smarter implementation
    r: Regex,
}

impl IgnorePattern {
    // TODO: I need a smarter implementation
    pub fn parse(pattern: &str) -> Self {
        let replaces = vec![
            (r"^\*\*$", r".+"),
            (r"^\*\*/", r"([^/]+/)_ast"),
            (r"/\*\*$", r"(/[^/]+)_ast"),
            (r"/\*\*/", r"/([^/]+/)_ast"),

            (r"^\*$", r"[^/]+"),
            (r"/\*$", r"/[^/]+"),
        ];

        let mut pattern = pattern.replace("_", "_und");
        pattern = pattern.replace("+", "_pls");
        pattern = pattern.replace(".", "_dot");
        pattern = pattern.replace("[", "_opn");
        pattern = pattern.replace("]", "_cls");

        for (bef, aft) in replaces.iter() {
            let bef = Regex::new(bef).unwrap();
            pattern = bef.replace_all(&pattern, *aft).to_string();
        }

        pattern = pattern.replace("_ast", "*");
        pattern = pattern.replace("_cls", "]");
        pattern = pattern.replace("_opn", "[");
        pattern = pattern.replace("_dot", "\\.");
        pattern = pattern.replace("_pls", "\\+");
        pattern = pattern.replace("_und", "_");
        IgnorePattern { r: Regex::new(&pattern).unwrap() }
    }

    // `path` must be a normalized, relative path
    pub fn is_match(&self, path: &str) -> bool {
        self.r.is_match(path)
    }
}
