use super::Index;
use crate::error::Error;
use ragit_api::JsonType;
use ragit_fs::{
    WriteMode,
    exists,
    read_string,
    remove_file,
    write_bytes,
};
use serde_json::Value;
use std::collections::HashMap;

pub type Path = String;
pub const METADATA_FILE_NAME: &str = "meta.json";

impl Index {
    pub fn get_meta_by_key(&self, key: String) -> Result<Option<String>, Error> {
        let meta_path = get_meta_path(&self.root_dir)?;

        if !exists(&meta_path) {
            return Ok(None);
        }

        let meta = load_meta(&meta_path)?;
        Ok(meta.get(&key).map(|v| v.to_string()))
    }

    pub fn get_all_meta(&self) -> Result<HashMap<String, String>, Error> {
        let meta_path = get_meta_path(&self.root_dir)?;

        if !exists(&meta_path) {
            return Ok(HashMap::new());
        }

        let meta = load_meta(&meta_path)?;
        Ok(meta)
    }

    pub fn set_meta_by_key(&self, key: String, value: String) -> Result<(), Error> {
        let meta_path = get_meta_path(&self.root_dir)?;

        if !exists(&meta_path) {
            save_meta(&meta_path, HashMap::new())?;
        }

        let mut meta = load_meta(&meta_path)?;
        meta.insert(key, value);
        save_meta(&meta_path, meta)
    }

    pub fn remove_meta_by_key(&self, key: String) -> Result<(), Error> {
        let meta_path = get_meta_path(&self.root_dir)?;

        if !exists(&meta_path) {
            return Ok(());
        }

        let mut meta = load_meta(&meta_path)?;
        meta.remove(&key);
        Ok(())
    }

    pub fn remove_all_meta(&self) -> Result<(), Error> {
        let meta_path = get_meta_path(&self.root_dir)?;

        if exists(&meta_path) {
            remove_file(&meta_path)?;
        }

        Ok(())
    }
}

fn get_meta_path(root_dir: &Path) -> Result<Path, Error> {
    Index::get_rag_path(
        root_dir,
        &METADATA_FILE_NAME.to_string(),
    )
}

fn load_meta(path: &Path) -> Result<HashMap<String, String>, Error> {
    let content = read_string(path)?;
    let j = serde_json::from_str::<Value>(&content)?;
    let Value::Object(obj) = j else { return Err(Error::JsonTypeError {
        expected: JsonType::Object,
        got: (&j).into(),
    }) };
    let mut result = HashMap::with_capacity(obj.len());

    for (key, value) in obj.iter() {
        match value.as_str() {
            Some(value) => {
                result.insert(key.to_string(), value.to_string());
            },
            None => {
                return Err(Error::JsonTypeError {
                    expected: JsonType::Object,
                    got: value.into(),
                });
            },
        }
    }

    Ok(result)
}

fn save_meta(path: &Path, meta: HashMap<String, String>) -> Result<(), Error> {
    Ok(write_bytes(
        path,
        &serde_json::to_vec_pretty(&meta.into_iter().map(
            |(key, value)| (key, Value::String(value))
        ).collect::<Value>())?,
        WriteMode::CreateOrTruncate,
    )?)
}
