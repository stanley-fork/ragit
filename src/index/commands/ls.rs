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

impl Index {
    /// `rag ls-chunks`
    ///
    /// It iterates all the chunks in the knowledge-base, which can be very expensive. If you know the uid of the chunk,
    /// use `get_chunk_by_uid` instead.
    pub fn list_chunks<Filter, Map, Sort, Key: Ord>(
        &self,
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Result<Vec<ChunkSchema>, Error> where Filter: Fn(&ChunkSchema) -> bool, Map: Fn(ChunkSchema) -> ChunkSchema, Sort: Fn(&ChunkSchema) -> Key {
        let mut result = vec![];

        for chunk_file in self.get_all_chunk_files()? {
            let chunk = chunk::load_from_file(&chunk_file)?;
            let chunk: ChunkSchema = chunk.into();

            if !filter(&chunk) {
                continue;
            }

            let chunk = map(chunk);
            result.push(chunk);
        }

        result.sort_by_key(sort_key);
        Ok(result)
    }

    /// `rag ls-files`
    ///
    /// It iterates all the files, which can be very expensive. If you know the uid or path of the file,
    /// use `get_file_schema` instead.
    pub fn list_files<Filter, Map, Sort, Key: Ord>(
        &self,
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Result<Vec<FileSchema>, Error> where Filter: Fn(&FileSchema) -> bool, Map: Fn(FileSchema) -> FileSchema, Sort: Fn(&FileSchema) -> Key {
        let mut result = vec![];

        for file in self.staged_files.iter() {
            let f = FileSchema {
                path: file.clone(),
                is_processed: false,
                ..FileSchema::dummy()
            };

            if filter(&f) {
                result.push(map(f));
            }
        }

        for (file, uid) in self.processed_files.iter() {
            let f = self.get_file_schema_worker(file.to_string(), *uid)?;

            if filter(&f) {
                result.push(map(f));
            }
        }

        result.sort_by_key(sort_key);
        Ok(result)
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
    pub fn list_images<Filter, Map, Sort, Key: Ord>(
        &self,
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Result<Vec<ImageSchema>, Error> where Filter: Fn(&ImageSchema) -> bool, Map: Fn(ImageSchema) -> ImageSchema, Sort: Fn(&ImageSchema) -> Key {
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

            result.push(map(image));
        }

        result.sort_by_key(sort_key);
        Ok(result)
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
