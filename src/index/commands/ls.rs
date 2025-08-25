//! Functions in this module iterates objects in the knowledge-base. It filters
//! and sorts the objects. There are 2 ways to do that:
//! 1. Load everything to memory and filter/sort them and return the objects.
//! 2. Just load uid and sort_key to memory, sort them and return the uids.
//!
//! The implementation was originally 1, but now it's 2 (except ls-models and --json mode).
//! My idea behind that is,
//! A. If the knowledge-base is small enough, 1 and 2 are both fast enough and nobody cares.
//! B. If the knowledge-base isn't small, but still can fit in memory, 1 is 2x faster than 2. It's bad.
//! C. If the knowledge-base is very big and cannot fit in memory, 1 will panic (OOM) but 2 will slowly run.
//!
//! That's why I think 2 is better than 1.
//!
//! In `--json` mode and `ls-models` command, it loads everything to memory because
//! 1. `serde_json` requires you to do so.
//! 2. There's no uid for models and `ls-models` is almost always small enough to fit into memory.

use super::Index;
use crate::chunk;
use crate::error::Error;
use crate::schema::{
    ChunkSchema,
    FileSchema,
    ImageSchema,
    ModelSchema,
    QueryTurnSchema,
};
use crate::uid::Uid;
use ragit_api::load_models;
use ragit_fs::{file_name, parent};

/// It's a return type of `Index::list_files`. It's the
/// smallest type that can uniquely identify a file. If it's
/// a processed file, it has `Uid` variant and if it's a
/// staged file, it's `StagedFile`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UidOrStagedFile {
    Uid(Uid),
    StagedFile(String),
}

impl Index {
    /// `rag ls-chunks`
    ///
    /// It iterates all the chunks in the knowledge-base, which can be very expensive. If you know the uid of the chunk,
    /// use `get_chunk_by_uid` instead.
    pub fn list_chunks<Filter, Sort, Key: Clone + Ord>(
        &self,
        filter: &Filter,
        sort_key: &Sort,
    ) -> Result<Vec<Uid>, Error> where Filter: Fn(&ChunkSchema) -> bool, Sort: Fn(&ChunkSchema) -> Key {
        let mut result = vec![];

        for chunk_file in self.get_all_chunk_files()? {
            let chunk = chunk::load_from_file(&chunk_file)?;
            let chunk: ChunkSchema = chunk.into();

            if !filter(&chunk) {
                continue;
            }

            let key = sort_key(&chunk);
            result.push((chunk.uid, key));
        }

        result.sort_by_key(|(_, key)| key.clone());
        Ok(result.into_iter().map(|(uid, _)| uid).collect())
    }

    /// `rag ls-files`
    ///
    /// It iterates all the files, which can be very expensive. If you know the uid or path of the file,
    /// use `get_file_schema` instead.
    pub fn list_files<Filter, Sort, Key: Clone + Ord>(
        &self,
        filter: &Filter,
        sort_key: &Sort,
    ) -> Result<Vec<UidOrStagedFile>, Error> where Filter: Fn(&FileSchema) -> bool, Sort: Fn(&FileSchema) -> Key {
        let mut result = vec![];

        for file in self.staged_files.iter() {
            let f = FileSchema {
                path: file.clone(),
                is_processed: false,
                ..FileSchema::dummy()
            };

            if !filter(&f) {
                continue;
            }

            let key = sort_key(&f);
            result.push((UidOrStagedFile::StagedFile(file.clone()), key));
        }

        for (file, uid) in self.processed_files.iter() {
            let f = self.get_file_schema_worker(file.to_string(), *uid)?;

            if !filter(&f) {
                continue;
            }

            let key = sort_key(&f);
            result.push((UidOrStagedFile::Uid(f.uid), key));
        }

        result.sort_by_key(|(_, key)| key.clone());
        Ok(result.into_iter().map(|(uid, _)| uid).collect())
    }

    /// `rag ls-models`
    pub fn list_models<Filter, Map, Sort, Key: Ord>(
        // `.ragit/models.json`
        models_at: &str,

        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Result<Vec<ModelSchema>, Error> where Filter: Fn(&ModelSchema) -> bool, Map: Fn(ModelSchema) -> ModelSchema, Sort: Fn(&ModelSchema) -> Key {
        let mut result = vec![];

        for model in load_models(models_at)? {
            if !filter(&model) {
                continue;
            }

            let model = map(model);
            result.push(model);
        }

        result.sort_by_key(sort_key);
        Ok(result)
    }

    /// `rag ls-images`
    ///
    /// It iterates all the images, which can be very expensive. If you know the uid of the image,
    /// use `get_image_schema` instead.
    pub fn list_images<Filter, Sort, Key: Clone + Ord>(
        &self,
        filter: &Filter,
        sort_key: &Sort,
    ) -> Result<Vec<Uid>, Error> where Filter: Fn(&ImageSchema) -> bool, Sort: Fn(&ImageSchema) -> Key {
        let mut result = vec![];

        for image in self.get_all_image_files()? {
            let image = self.get_image_schema(
                Uid::from_prefix_and_suffix(
                    &file_name(&parent(&image)?)?,
                    &file_name(&image)?,
                )?,
                false,
            )?;

            if !filter(&image) {
                continue;
            }

            let key = sort_key(&image);
            result.push((image.uid, key));
        }

        result.sort_by_key(|(_, key)| key.clone());
        Ok(result.into_iter().map(|(uid, _)| uid).collect())
    }

    /// `rag ls-queries`
    ///
    /// It iterates all the queries, which can be very expensive. If you know the uid of the query,
    /// use `get_query_schema` instead.
    pub fn list_queries<Filter, Sort, Key: Clone + Ord>(
        &self,
        filter: &Filter,
        sort_key: &Sort,
    ) -> Result<Vec<Uid>, Error> where Filter: Fn(&[QueryTurnSchema]) -> bool, Sort: Fn(&[QueryTurnSchema]) -> Key {
        let mut result = vec![];

        for query_file in self.get_all_query_history_files()? {
            let query_uid = Uid::from_prefix_and_suffix(
                &file_name(&parent(&query_file)?)?,
                &file_name(&query_file)?,
            )?;
            let query = self.get_query_schema(query_uid)?;

            if !filter(&query) {
                continue;
            }

            let key = sort_key(&query);
            result.push((query_uid, key));
        }

        result.sort_by_key(|(_, key)| key.clone());
        Ok(result.into_iter().map(|(uid, _)| uid).collect())
    }
}
