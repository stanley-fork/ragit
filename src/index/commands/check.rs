use super::Index;
use crate::{ApiConfigRaw, QueryConfig};
use crate::chunk;
use crate::error::Error;
use crate::index::{BuildConfig, CHUNK_DIR_NAME, CHUNK_INDEX_DIR_NAME, IMAGE_DIR_NAME, INDEX_DIR_NAME, tfidf, xor_sha3};
use json::JsonValue;
use ragit_api::{JsonType, get_type};
use ragit_fs::{
    basename,
    exists,
    extension,
    file_name,
    join3,
    read_bytes,
    read_dir,
    read_string,
    set_extension,
};
use std::collections::{HashMap, HashSet};

impl Index {
    /// - Check A: For each chunk file, a tfidf file either 1) not exist at all or 2) complete
    /// - Check B: `get_chunk_file_by_index(uid)` gives a correct result for all chunks.
    /// - Check C: `self.chunk_files` has the correct number of chunks for each chunk file.
    /// - Check D: `self.chunk_count` has the correct number.
    /// - Check E: `self.processed_files` is correct.
    /// - Check F: Entries in `.ragit/chunk_index/*.json` points to a valid chunk.
    /// - Check G: Images in chunks are all in `.ragit/images` and vice versa.
    /// - Check H: Images in `.ragit/images` are not corrupted. They all must have an attached json file.
    /// - Check I: Config files are not broken.
    /// - Check J: A name of a chunk file is an xor of its chunks' uids.
    pub fn check(&self, recursive: bool) -> Result<(), Error> {
        let mut chunk_count = 0;
        let mut processed_files = HashSet::with_capacity(self.processed_files.len());
        let mut chunk_index = HashMap::with_capacity(self.chunk_count);  // HashMap<uid, chunk_index>
        let mut images = HashMap::new();  // HashMap<image_id, has_found>

        for chunk_file in read_dir(&join3(
            &self.root_dir,
            &INDEX_DIR_NAME,
            &CHUNK_DIR_NAME,
        )?)? {
            if extension(&chunk_file)?.unwrap_or(String::new()) != "chunks" {
                continue;
            }

            if !self.chunk_files.contains_key(&file_name(&chunk_file)?) {  // Check C
                return Err(Error::BrokenIndex(format!("`{chunk_file}` exists, but is not included in `index.json`")));
            }
        }

        for chunk_file in self.chunk_files_real_path() {
            let chunks = chunk::load_from_file(&chunk_file)?;
            let tfidf_path = set_extension(&chunk_file, "tfidf")?;
            let tfidfs = if exists(&tfidf_path) {
                Some(tfidf::load_from_file(&tfidf_path)?)
            } else {
                None
            };
            let mut chunks_in_tfidf = HashSet::new();

            if let Some(tfidfs) = &tfidfs {
                if chunks.len() != tfidfs.len() {  // Check A
                    return Err(Error::BrokenIndex(format!(
                        "There are {} chunks in `{}`, but {} processed_docs in `{}`.",
                        chunks.len(),
                        basename(&chunk_file).unwrap(),
                        tfidfs.len(),
                        basename(&tfidf_path).unwrap(),
                    )));
                }

                for processed_doc in tfidfs.iter() {
                    match &processed_doc.chunk_uid {
                        Some(uid) => {
                            let new = chunks_in_tfidf.insert(uid.clone());

                            if !new {
                                return Err(Error::BrokenIndex(format!(
                                    "There are more than one processed_doc whose chunk_uid is {uid} in `{}`.",
                                    basename(&tfidf_path).unwrap(),
                                )));
                            }
                        },
                        None => {
                            return Err(Error::BrokenIndex(format!(
                                "There's a processed_doc whose chunk_uid is None in `{}`.",
                                basename(&tfidf_path).unwrap(),
                            )));
                        },
                    }
                }
            }

            let chunk_file_name = file_name(&chunk_file)?;
            chunk_count += chunks.len();

            match self.chunk_files.get(&chunk_file_name) {
                Some(n) => {
                    if *n != chunks.len() {  // Check C
                        return Err(Error::BrokenIndex(format!(
                            "`index.json` says there are {n} chunks in `{}`, but there actually are {} chunks in the file.",
                            basename(&chunk_file).unwrap(),
                            chunks.len(),
                        )));
                    }
                },
                None => {  // Check C
                    return Err(Error::BrokenIndex(format!(
                        "self.chunk_files does not include {:?}.",
                        chunk_file_name,
                    )));
                },
            }

            let mut xor_uids = format!("{:064x}", 0);

            for chunk in chunks.iter() {
                processed_files.insert(chunk.file.clone());
                let (root_dir, chunk_file_by_index) = self.get_chunk_file_by_index(&chunk.uid)?;
                chunk_index.insert(chunk.uid.clone(), chunk_file_by_index.clone());
                xor_uids = xor_sha3(
                    &xor_uids,
                    &chunk.uid,
                )?;

                if root_dir != self.root_dir {  // Check B
                    return Err(Error::BrokenIndex(format!(
                        "`self.root_dir` is `{}`, but chunk {} says its root_dir is `{root_dir}`.",
                        self.root_dir,
                        chunk.uid,
                    )));
                }

                if chunk_file_name != chunk_file_by_index {  // Check B
                    return Err(Error::BrokenIndex(format!(
                        "chunk_index file says {} belongs to `{chunk_file_by_index}`, but it actually belongs to `{}`",
                        chunk.uid,
                        basename(&chunk_file_name).unwrap(),
                    )));
                }

                if tfidfs.is_some() && !chunks_in_tfidf.contains(&chunk.uid) {  // Check A
                    return Err(Error::BrokenIndex(format!(
                        "chunk_file {chunk_file_name} has a tfidf index, but there's no processed_doc for chunk {}.",
                        chunk.uid,
                    )));
                }

                for image in chunk.images.iter() {
                    images.insert(image.to_string(), false);
                }
            }

            if chunk_file_name != xor_uids {  // Check J
                return Err(Error::BrokenIndex(format!(
                    "chunk_file {chunk_file_name}'s xor-ed value is {xor_uids}",
                )));
            }
        }

        for file in processed_files.iter() {
            if !self.processed_files.contains_key(file) && self.curr_processing_file != Some(file.to_string()) {  // Check E
                return Err(Error::BrokenIndex(format!(
                    "There's a chunk of {file:?}, but we cannot find the file in self.processed_files and self.curr_processing_file.",
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
                                    return Err(Error::BrokenIndex(format!(
                                        "A chunk_index is outdated: the index says {uid} belongs to {chunk_file}, but it actually belongs to {chunk_file_}",
                                    )));
                                },
                                None => {  // Check F
                                    return Err(Error::BrokenIndex(format!(
                                        "A chunk_index is outdated: the index says {uid} belongs to {chunk_file}, but such chunk does not exist.",
                                    )));
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
            if extension(&image_file)?.unwrap_or(String::new()) == "json" {
                continue;
            }

            let image_file_hash = file_name(&image_file)?;
            let image_description_path = set_extension(&image_file, "json")?;

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

            let image_description = read_string(&image_description_path)?;

            match json::parse(&image_description) {
                Ok(JsonValue::Object(_)) => {},
                _ => {  // Check H
                    return Err(Error::BrokenIndex(format!("{image_description_path:?}'s schema is not what we have expected.")));
                },
            }
        }

        for (image_id, has_found) in images.iter() {  // Check G
            if !*has_found {
                return Err(Error::BrokenIndex(format!("{image_id:?} not found in `.ragit/images/`")));
            }
        }

        if recursive {
            for external_index in self.external_indexes.iter() {
                external_index.check(recursive)?;
            }
        }

        if (self.processed_files.len() + self.curr_processing_file.is_some() as usize) != processed_files.len() {  // Check E
            return Err(Error::BrokenIndex(format!(
                "self.processed_files.len() = {}\nself.curr_processing_file = {:?}\nprocessed_files.len() = {}",
                self.processed_files.len(),
                self.curr_processing_file,
                processed_files.len(),
            )));
        }

        else if chunk_count != self.chunk_count {  // Check D
            return Err(Error::BrokenIndex(format!(
                "chunk_count = {chunk_count}\nself.chunk_count = {}",
                self.chunk_count,
            )));
        }

        // Check I
        serde_json::from_str::<BuildConfig>(
            &read_string(&self.get_build_config_path()?)?,
        )?;
        serde_json::from_str::<QueryConfig>(
            &read_string(&self.get_query_config_path()?)?,
        )?;
        let api_config_raw = serde_json::from_str::<ApiConfigRaw>(
            &read_string(&self.get_api_config_path()?)?,
        )?;
        self.init_api_config(&api_config_raw)?;

        // Extra check: It checks whether the keys in the config files are unique.
        let mut keys = HashSet::new();

        for path in [
            self.get_build_config_path()?,
            self.get_api_config_path()?,
            self.get_query_config_path()?,
        ] {
            let j = read_string(&path)?;
            let j = json::parse(&j)?;

            for (key, _) in j.entries() {
                if keys.contains(key) {
                    return Err(Error::BrokenIndex(format!("Key conflict in config file {path:?}: {key:?}")));
                }

                keys.insert(key.to_string());
            }
        }

        Ok(())
    }
}
