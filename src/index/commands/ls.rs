use super::Index;
use crate::chunk;
use crate::error::Error;
use crate::schema::{ChunkSchema, FileSchema, ImageSchema, ModelSchema};
use crate::uid::Uid;
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
        // `filter` is applied before `map`
        filter: &Filter,
        map: &Map,
        sort_key: &Sort,
    ) -> Vec<ModelSchema> where Filter: Fn(&ModelSchema) -> bool, Map: Fn(ModelSchema) -> ModelSchema, Sort: Fn(&ModelSchema) -> Key {
        let mut result = vec![];

        for model in ragit_api::ChatModel::all_kinds() {
            let api_provider = model.get_api_provider();
            let ls_model = ModelSchema {
                name: model.to_human_friendly_name().to_string(),
                api_provider: api_provider.as_str().to_string(),
                api_key_env_var: api_provider.api_key_env_var().map(|v| v.to_string()),
                can_read_images: model.can_read_images(),
                dollars_per_1b_input_tokens: model.dollars_per_1b_input_tokens(),
                dollars_per_1b_output_tokens: model.dollars_per_1b_output_tokens(),
                explanation: model.explanation().to_string(),
            };

            if !filter(&ls_model) {
                continue;
            }

            let ls_model = map(ls_model);
            result.push(ls_model);
        }

        result.sort_by_key(sort_key);
        result
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
}
