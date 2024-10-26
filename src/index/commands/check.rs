use super::Index;
use crate::chunk::{self, Uid};
use crate::error::Error;
use crate::index::tfidf;
use ragit_fs::{basename, set_ext};
use std::collections::HashSet;

impl Index {
    // TODO: check images
    pub fn check(&self, recursive: bool) -> Result<(), Error> {
        let mut chunk_count = 0;
        let mut processed_files = HashSet::with_capacity(self.processed_files.len());

        for chunk_file in self.chunk_files_real_path() {
            let chunks = chunk::load_from_file(&chunk_file)?;
            let tfidfs = tfidf::load_from_file(&set_ext(&chunk_file, "tfidf")?)?;
            let mut chunks_in_tfidf = HashSet::with_capacity(tfidfs.len());

            if chunks.len() != tfidfs.len() {
                return Err(Error::BrokenIndex(format!(
                    "chunks.len() = {}\ntfidfs.len() = {}",
                    chunks.len(),
                    tfidfs.len(),
                )));
            }

            for processed_docs in tfidfs.iter() {
                let mut curr_chunk_uid: Option<Uid> = None;

                if processed_docs.len() == 0 {
                    return Err(Error::BrokenIndex(format!(
                        "processed_docs.len() == 0",
                    )));
                }

                for processed_doc in processed_docs.values() {
                    if processed_doc.chunk_uid.is_none() {
                        return Err(Error::BrokenIndex(format!(
                            "procssed_doc.chunk_uid.is_none()",
                        )));
                    }

                    match &curr_chunk_uid {
                        Some(uid) => {
                            if processed_doc.chunk_uid.clone().unwrap() != uid.clone() {
                                return Err(Error::BrokenIndex(format!(
                                    "processed_doc.chunk_uid.unwrap() = {:?}\nuid={uid:?}",
                                    processed_doc.chunk_uid.clone().unwrap(),
                                )));
                            }
                        },
                        None => {
                            curr_chunk_uid = Some(processed_doc.chunk_uid.clone().unwrap());
                            chunks_in_tfidf.insert(processed_doc.chunk_uid.clone().unwrap());
                        },
                    }
                }
            }

            if chunks_in_tfidf.len() != chunks.len() {
                return Err(Error::BrokenIndex(format!(
                    "chunks_in_tfidf.len() = {}\nchunks.len() = {}",
                    chunks_in_tfidf.len(),
                    chunks.len(),
                )));
            }

            let chunk_file_basename = basename(&chunk_file)?;
            chunk_count += chunks.len();

            match self.chunk_files.get(&chunk_file_basename) {
                Some(n) => {
                    if *n != chunks.len() {
                        return Err(Error::BrokenIndex(format!(
                            "self.chunk_files.get({:?}) = Some({n})\nchunks.len() = {}",
                            chunk_file_basename,
                            chunks.len(),
                        )));
                    }
                },
                None => {
                    return Err(Error::BrokenIndex(format!(
                        "self.chunk_files.get({:?}) = None",
                        chunk_file_basename,
                    )));
                },
            }

            for chunk in chunks.iter() {
                processed_files.insert(chunk.file.clone());
                let (root_dir, chunk_file_by_index) = self.get_chunk_file_by_index(&chunk.uid)?;

                if root_dir != self.root_dir {
                    return Err(Error::BrokenIndex(format!(
                        "root_dir = {root_dir}\nself.root_dir = {}",
                        self.root_dir,
                    )));
                }

                if chunk_file_basename != chunk_file_by_index {
                    return Err(Error::BrokenIndex(format!(
                        "chunk_file_basename = {chunk_file_basename:?}\nself.get_chunk_file_by_index({:?})? = {chunk_file_by_index}",
                        chunk.uid,
                    )));
                }

                if !chunks_in_tfidf.contains(&chunk.uid) {
                    return Err(Error::BrokenIndex(format!(
                        "!chunks_in_tfidf.contains({:?})",
                        chunk.uid,
                    )));
                }
            }
        }

        for file in processed_files.iter() {
            if !self.processed_files.contains_key(file) && self.curr_processing_file != Some(file.to_string()) {
                return Err(Error::BrokenIndex(format!(
                    "!self.processed_files.contains_key({file:?}) && {:?} != Some({file:?})",
                    self.curr_processing_file,
                )));
            }
        }

        if recursive {
            for external_index in self.external_indexes.iter() {
                external_index.check(recursive)?;
            }
        }

        if (self.processed_files.len() + if self.curr_processing_file.is_some() { 1 } else { 0 }) != processed_files.len() {
            Err(Error::BrokenIndex(format!(
                "self.processed_files.len() = {}\nself.curr_processing_file = {:?}\nprocessed_files.len() = {}",
                self.processed_files.len(),
                self.curr_processing_file,
                processed_files.len(),
            )))
        }

        else if chunk_count != self.chunk_count {
            Err(Error::BrokenIndex(format!(
                "chunk_count = {chunk_count}\nself.chunk_count = {}",
                self.chunk_count,
            )))
        }

        else {
            Ok(())
        }
    }
}
