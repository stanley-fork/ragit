use super::Index;
use crate::error::Error;
use crate::index::{CHUNK_DIR_NAME, IIStatus};
use ragit_fs::{exists, get_relative_path, remove_file, set_extension};
use std::collections::HashSet;

pub type Path = String;

#[derive(Clone, Copy, Default)]
pub struct RemoveResult {
    pub staged: usize,
    pub processed: usize,
}

impl Index {
    pub fn remove_file(
        &mut self,
        path: Path,
        dry_run: bool,
        recursive: bool,
        auto: bool,
        staged: bool,
        processed: bool,
    ) -> Result<RemoveResult, Error> {
        let mut rel_path = get_relative_path(&self.root_dir, &path)?;
        let (mut staged_candidates, mut processed_candidates) = if recursive {
            if !rel_path.ends_with("/") {
                rel_path = format!("{rel_path}/");
            }

            let mut staged_candidates = vec![];
            let mut processed_candidates = vec![];

            // `--all`
            if rel_path == "/" {
                if staged {
                    staged_candidates = self.staged_files.iter().map(|f| f.to_string()).collect();
                }

                if processed {
                    processed_candidates = self.processed_files.keys().map(|f| f.to_string()).collect();
                }
            }

            else {
                if staged {
                    for file in self.staged_files.iter() {
                        if file.starts_with(&rel_path) {
                            staged_candidates.push(file.to_string());
                        }
                    }
                }

                if processed {
                    for file in self.processed_files.keys() {
                        if file.starts_with(&rel_path) {
                            processed_candidates.push(file.to_string());
                        }
                    }
                }
            }

            (staged_candidates, processed_candidates)
        } else {
            let staged_candidates = if staged && self.staged_files.contains(&rel_path) {
                vec![rel_path.clone()]
            } else {
                vec![]
            };
            let processed_candidates = if processed && self.processed_files.contains_key(&rel_path) {
                vec![rel_path.clone()]
            } else {
                vec![]
            };

            (staged_candidates, processed_candidates)
        };

        if staged_candidates.is_empty() && processed_candidates.is_empty() && !recursive {
            return Err(Error::NoSuchFile { path: Some(path), uid: None });
        }

        if auto {
            let mut staged_candidates_new = vec![];
            let mut processed_candidates_new = vec![];

            for file in staged_candidates.into_iter() {
                if !exists(&Index::get_data_path(&self.root_dir, &file)?) {
                    staged_candidates_new.push(file)
                }
            }

            for file in processed_candidates.into_iter() {
                if !exists(&Index::get_data_path(&self.root_dir, &file)?) {
                    processed_candidates_new.push(file)
                }
            }

            staged_candidates = staged_candidates_new;
            processed_candidates = processed_candidates_new;
        }

        // TODO: `if !dry_run {}` must not return, but it's extremely hard to do so
        if !dry_run {
            let staged_candidates: HashSet<_> = staged_candidates.iter().collect();
            self.staged_files = self.staged_files.iter().filter(
                |file| !staged_candidates.contains(file)
            ).map(
                |file| file.to_string()
            ).collect();

            self.ii_status = IIStatus::Outdated;

            for file in processed_candidates.iter() {
                match self.processed_files.get(file).map(|uid| *uid) {
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

                        self.processed_files.remove(file).unwrap();
                        self.remove_file_index(file_uid)?;
                    },
                    _ => {},
                }
            }
        }

        Ok(RemoveResult {
            staged: staged_candidates.len(),
            processed: processed_candidates.len(),
        })
    }
}

impl std::ops::AddAssign<Self> for RemoveResult {
    fn add_assign(&mut self, rhs: Self) {
        self.staged += rhs.staged;
        self.processed += rhs.processed;
    }
}
