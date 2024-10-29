use super::Index;
use crate::chunk;
use crate::error::Error;
use crate::index::{CHUNK_INDEX_DIR_NAME, IMAGE_DIR_NAME, tfidf};
use json::JsonValue;
use ragit_api::{JsonType, get_type};
use ragit_fs::{file_name, read_bytes, read_dir, read_string, set_ext};
use std::collections::{HashMap, HashSet};

impl Index {
    /// Check A: Every chunk file has a corresponding tfidf file, and the tfidf file has data for all the chunks in the chunk file.
    /// Check B: `get_chunk_file_by_index(uid)` gives a correct result for all chunks.
    /// Check C: `self.chunk_files` has the correct number of chunks for each chunk file.
    /// Check D: `self.chunk_count` has the correct number.
    /// Check E: `self.processed_files` is correct.
    /// Check F: Entries in `.rag_index/chunk_index/*.json` points to a valid chunk.
    /// Check G: Images in chunks are all in `.rag_index/images` and vice versa.
    /// Check H: Images in `.rag_index/images` are not corrupted.
    pub fn check(&self, recursive: bool) -> Result<(), Error> {
        let mut chunk_count = 0;
        let mut processed_files = HashSet::with_capacity(self.processed_files.len());
        let mut chunk_index = HashMap::with_capacity(self.chunk_count);  // HashMap<uid, chunk_index>
        let mut images = HashMap::new();  // HashMap<image_id, has_found>

        for chunk_file in self.chunk_files_real_path() {
            let chunks = chunk::load_from_file(&chunk_file)?;
            let tfidfs = tfidf::load_from_file(&set_ext(&chunk_file, "tfidf")?)?;
            let mut chunks_in_tfidf = HashSet::with_capacity(tfidfs.len());

            if chunks.len() != tfidfs.len() {  // Check A
                return Err(Error::BrokenIndex(format!(
                    "chunks.len() = {}\ntfidfs.len() = {}",
                    chunks.len(),
                    tfidfs.len(),
                )));
            }

            for processed_doc in tfidfs.iter() {
                match &processed_doc.chunk_uid {
                    Some(uid) => {
                        chunks_in_tfidf.insert(uid.clone());
                    },
                    None => {
                        return Err(Error::BrokenIndex(format!(
                            "processed_doc.chunk_uid.is_none()",
                        )));
                    },
                }
            }

            if chunks_in_tfidf.len() != chunks.len() {  // Check A
                return Err(Error::BrokenIndex(format!(
                    "chunks_in_tfidf.len() = {}\nchunks.len() = {}",
                    chunks_in_tfidf.len(),
                    chunks.len(),
                )));
            }

            let chunk_file_name = file_name(&chunk_file)?;
            chunk_count += chunks.len();

            match self.chunk_files.get(&chunk_file_name) {
                Some(n) => {
                    if *n != chunks.len() {  // Check C
                        return Err(Error::BrokenIndex(format!(
                            "self.chunk_files.get({:?}) = Some({n})\nchunks.len() = {}",
                            chunk_file_name,
                            chunks.len(),
                        )));
                    }
                },
                None => {  // Check C
                    return Err(Error::BrokenIndex(format!(
                        "self.chunk_files.get({:?}) = None",
                        chunk_file_name,
                    )));
                },
            }

            for chunk in chunks.iter() {
                processed_files.insert(chunk.file.clone());
                let (root_dir, chunk_file_by_index) = self.get_chunk_file_by_index(&chunk.uid)?;
                chunk_index.insert(chunk.uid.clone(), chunk_file_by_index.clone());

                if root_dir != self.root_dir {  // Check B
                    return Err(Error::BrokenIndex(format!(
                        "root_dir = {root_dir}\nself.root_dir = {}",
                        self.root_dir,
                    )));
                }

                if chunk_file_name != chunk_file_by_index {  // Check B
                    return Err(Error::BrokenIndex(format!(
                        "chunk_file_name = {chunk_file_name:?}\nself.get_chunk_file_by_index({:?})? = {chunk_file_by_index}",
                        chunk.uid,
                    )));
                }

                if !chunks_in_tfidf.contains(&chunk.uid) {  // Check A
                    return Err(Error::BrokenIndex(format!(
                        "!chunks_in_tfidf.contains({:?})",
                        chunk.uid,
                    )));
                }

                for image in chunk.images.iter() {
                    images.insert(image.to_string(), false);
                }
            }
        }

        for file in processed_files.iter() {
            if !self.processed_files.contains_key(file) && self.curr_processing_file != Some(file.to_string()) {  // Check E
                return Err(Error::BrokenIndex(format!(
                    "!self.processed_files.contains_key({file:?}) && {:?} != Some({file:?})",
                    self.curr_processing_file,
                )));
            }
        }

        for chunk_index_file in read_dir(&Index::get_rag_path(
            &self.root_dir,
            &CHUNK_INDEX_DIR_NAME.to_string(),
        ))? {
            let j = read_string(&chunk_index_file)?;
            let j = json::parse(&j)?;

            match &j {
                JsonValue::Object(obj) => {
                    for (uid, chunk_file) in obj.iter() {
                        match chunk_file.as_str() {
                            Some(chunk_file) => match chunk_index.get(uid) {
                                Some(chunk_file_) if chunk_file != chunk_file_ => {  // Check F
                                    return Err(Error::BrokenIndex(format!("chunk_index.get({uid:?}) = {chunk_file_:?}\nchunk_file = {chunk_file:?}")));
                                },
                                None => {  // Check F
                                    return Err(Error::BrokenIndex(format!("chunk_index.get({uid:?}) = None")));
                                },
                                _ => {},
                            },
                            None => {
                                return Err(Error::JsonTypeError {
                                    expected: JsonType::String,
                                    got: get_type(chunk_file),
                                });
                            },
                        }
                    }
                },
                _ => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: get_type(&j),
                    });
                },
            }
        }

        for image_file in read_dir(&Index::get_rag_path(
            &self.root_dir,
            &IMAGE_DIR_NAME.to_string(),
        ))? {
            let image_file_hash = file_name(&image_file)?;
            match images.get_mut(&image_file_hash) {
                Some(has_found) => { *has_found = true; },
                None => {  // Check G
                    return Err(Error::BrokenIndex(format!("{image_file_hash:?} not found in any chunk")));
                },
            }

            let image_bytes = read_bytes(&image_file)?;
            image::load_from_memory_with_format(  // Check H
                &image_bytes,
                image::ImageFormat::Png,
            )?;
        }

        for (image_id, has_found) in images.iter() {  // Check G
            if !*has_found {
                return Err(Error::BrokenIndex(format!("{image_id:?} not found in `.rag_index/images/`")));
            }
        }

        if recursive {
            for external_index in self.external_indexes.iter() {
                external_index.check(recursive)?;
            }
        }

        if (self.processed_files.len() + self.curr_processing_file.is_some() as usize) != processed_files.len() {  // Check E
            Err(Error::BrokenIndex(format!(
                "self.processed_files.len() = {}\nself.curr_processing_file = {:?}\nprocessed_files.len() = {}",
                self.processed_files.len(),
                self.curr_processing_file,
                processed_files.len(),
            )))
        }

        else if chunk_count != self.chunk_count {  // Check D
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
