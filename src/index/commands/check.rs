use super::Index;
use crate::{ApiConfigRaw, QueryConfig};
use crate::chunk;
use crate::error::Error;
use crate::index::{BuildConfig, IMAGE_DIR_NAME, tfidf};
use crate::uid::{self, Uid};
use json::JsonValue;
use ragit_fs::{
    basename,
    exists,
    extension,
    file_name,
    parent,
    read_bytes,
    read_dir,
    read_string,
    set_extension,
};
use std::collections::{HashMap, HashSet};

impl Index {
    /// - Check A: For each chunk file,
    ///   - Check A-0: the chunk is not corrupted.
    ///   - Check A-1: the file it points to is in `self.processed_files`.
    /// - Check B: For each file index,
    ///   - Check B-0: its chunk uids point to a real chunk and the chunk points to this file.
    ///   - Check B-1: `self.processed_files` has an entry for the file.
    ///   - Check B-2: All the files in `self.processed_files` have an index.
    /// - Check C: Files in `self.processed_files + self.staged_files + self.curr_processing_file` are unique.
    /// - Check D: `self.chunk_count` has a correct number.
    /// - Check E: Images in chunks are in `.ragit/images`.
    ///   - If there's and image that belongs to no chunks, that's not an error. Just run `rag gc --images`. (TODO: not implemented yet)
    /// - Check F: Images in `.ragit/images` are not corrupted, and has a proper description file.
    /// - Check G: Config files are not broken.
    pub fn check(&self, recursive: bool) -> Result<(), Error> {
        let mut images = HashMap::new();
        let mut chunks_to_files = HashMap::with_capacity(self.chunk_count);
        let mut processed_files = HashSet::with_capacity(self.processed_files.len());
        let uids_to_files = self.processed_files.iter().map(|(file, uid)| (uid.to_string(), file.to_string())).collect::<HashMap<_, _>>();
        let mut file_uid_checks = uids_to_files.keys().map(|uid| (uid.to_string(), false /* exists */)).collect::<HashMap<_, _>>();
        let mut chunk_count = 0;

        for chunk_file in self.get_all_chunk_files()? {
            let chunk_prefix = basename(&parent(&chunk_file)?)?;
            let chunk_suffix = file_name(&chunk_file)?;
            let chunk_uid = Uid::from_prefix_and_suffix(&chunk_prefix, &chunk_suffix)?;
            let chunk = chunk::load_from_file(&chunk_file)?;

            if chunk_uid != chunk.uid {  // Check A-0
                return Err(Error::BrokenIndex(format!("Corrupted chunk: `{chunk_file}`'s uid is supposed to be `{chunk_uid}`, but is `{}`.", chunk.uid)));
            }

            chunks_to_files.insert(chunk_uid, (chunk.file.to_string(), chunk.index));
            processed_files.insert(chunk.file.to_string());
            chunk_count += 1;

            for image in chunk.images.iter() {
                images.insert(image.to_string(), false /* exists */);
            }

            let tfidf_file = set_extension(&chunk_file, "tfidf")?;

            if exists(&tfidf_file) {
                tfidf::load_from_file(&tfidf_file)?;
            }
        }

        for processed_file in processed_files.iter() {
            if !self.processed_files.contains_key(processed_file) {  // Check A-1
                return Err(Error::BrokenIndex(format!("There's a chunk of `{processed_file}`, but self.processed_files does not have its entry.")));
            }
        }

        let mut all_files = HashSet::with_capacity(self.staged_files.len() + self.processed_files.len() + 1);

        for file in self.staged_files.iter().chain(self.processed_files.keys()).chain(self.curr_processing_file.iter()) {
            if all_files.contains(file) {
                return Err(Error::BrokenIndex(format!("`{file}` appears multiple times in the index.")));
            }

            all_files.insert(file.to_string());
        }

        for file_index in self.file_index_real_path()? {
            let uid_prefix = basename(&parent(&file_index)?)?;
            let uid_suffix = file_name(&file_index)?;
            let file_uid = format!("{uid_prefix}{uid_suffix}");
            let file_name = match uids_to_files.get(&file_uid) {
                Some(file_name) => file_name.to_string(),
                None => {  // Check B-1
                    return Err(Error::BrokenIndex(format!("There's a file_index for `{file_uid}`, but self.processed_files does not have an entry with such hash value.")));
                },
            };

            match file_uid_checks.get_mut(&file_uid) {
                Some(exists) => { *exists = true; },
                None => unreachable!(),  // Check B-1, already checked
            }

            for (index1, uid) in uid::load_from_file(&file_index)?.iter().enumerate() {
                match chunks_to_files.get(uid) {
                    Some((file_name_from_chunk, index2)) => {
                        if &file_name != file_name_from_chunk {  // Check B-0
                            return Err(Error::BrokenIndex(format!("`{file_index}`'s file name is `{file_name}` and it has a chunk `{uid}`. But the chunk points to `{file_name_from_chunk}`.")));
                        }

                        // Extra check: chunk uids in a file_index must be in a correct order
                        if index1 != *index2 {
                            return Err(Error::BrokenIndex(format!("`{file_index}`'s {index1}th chunk uid is `{uid}`, but the chunk's index is {index2}.")));
                        }
                    },
                    None => {  // Check B-0
                        return Err(Error::BrokenIndex(format!("`{file_index}` has a chunk `{uid}`, but there's no such chunk in `.ragit/chunks`.")));
                    },
                }
            }
        }

        for (file_uid, exists) in file_uid_checks.iter() {
            if !*exists {  // Check B-2
                let file_name = uids_to_files.get(file_uid).unwrap();
                return Err(Error::BrokenIndex(format!("`{file_name}` doesn't have an index.")));
            }
        }

        if chunk_count != self.chunk_count {  // Check D
            return Err(Error::BrokenIndex(format!("self.chunk_count is {}, but the actual number is {chunk_count}", self.chunk_count)));
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
                Some(exists) => { *exists = true; },
                None => {
                    // NOTE: it's not an error. see the comments above.
                },
            }

            let image_bytes = read_bytes(&image_file)?;
            image::load_from_memory_with_format(  // Check F
                &image_bytes,
                image::ImageFormat::Png,
            )?;
            let image_description = read_string(&image_description_path)?;

            match json::parse(&image_description) {
                Ok(JsonValue::Object(_)) => {},
                _ => {  // Check F
                    return Err(Error::BrokenIndex(format!("`{image_file}` exists, but `{image_description_path}` does not exist.")));
                },
            }
        }

        for (image_file_hash, exists) in images.iter() {
            if !*exists {  // Check E
                return Err(Error::BrokenIndex(format!("`{image_file_hash}.png` not found.")));
            }
        }

        // Check G
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

        if recursive {
            for ext in self.external_indexes.iter() {
                ext.check(recursive)?;
            }
        }

        Ok(())
    }
}
