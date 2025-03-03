use super::{BuildConfig, Index};
use crate::{ApiConfig, QueryConfig};
use crate::error::Error;
use ragit_api::JsonType;
use ragit_fs::{WriteMode, read_string, write_bytes, write_string};
use serde_json::Value;
use std::collections::HashMap;

impl Index {
    pub fn get_config_by_key(&self, key: String) -> Result<Value, Error> {
        for path in [
            self.get_build_config_path()?,
            self.get_api_config_path()?,
            self.get_query_config_path()?,
        ] {
            let j = read_string(&path)?;
            let j = serde_json::from_str::<Value>(&j)?;

            match j {
                Value::Object(obj) => match obj.get(&key) {
                    Some(v) => { return Ok(v.clone()) },
                    _ => {},
                },
                _ => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: (&j).into(),
                    });
                },
            }
        }

        Err(Error::InvalidConfigKey(key))
    }

    /// It returns `Vec` instead of `HashMap` or `Json` since `Vec` is easier to sort by key.
    /// It does not sort the keys. It's your responsibility to do that.
    pub fn get_all_configs(&self) -> Result<Vec<(String, Value)>, Error> {
        let mut result = vec![];

        for path in [
            self.get_build_config_path()?,
            self.get_api_config_path()?,
            self.get_query_config_path()?,
        ] {
            let j = read_string(&path)?;
            let j = serde_json::from_str::<Value>(&j)?;

            match j {
                Value::Object(obj) => {
                    for (k, v) in obj.iter() {
                        result.push((k.to_string(), v.clone()));
                    }
                },
                _ => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: (&j).into(),
                    });
                },
            }
        }

        Ok(result)
    }

    /// It returns the previous value, if exists.
    pub fn set_config_by_key(&mut self, key: String, value: String) -> Result<Option<String>, Error> {
        // if `set_config_by_key` fails, it has to revert the json files before returning error
        let mut tmp_json_cache = HashMap::new();

        for json_file in [
            self.get_build_config_path()?,
            self.get_api_config_path()?,
            self.get_query_config_path()?,
        ] {
            tmp_json_cache.insert(
                json_file.clone(),
                read_string(&json_file)?,
            );
        }

        match self.set_config_by_key_worker(key, value) {
            Ok(previous_value) => Ok(previous_value),
            Err(e) => {
                for (path, content) in tmp_json_cache.iter() {
                    write_string(
                        path,
                        content,
                        WriteMode::CreateOrTruncate,
                    )?;
                }

                Err(e)
            },
        }
    }

    fn set_config_by_key_worker(&mut self, key: String, value: String) -> Result<Option<String>, Error> {  // returns the previous value, if exists
        let mut updated = false;
        let mut previous_value = None;

        for path in [
            self.get_build_config_path()?,
            self.get_api_config_path()?,
            self.get_query_config_path()?,
        ] {
            let j = read_string(&path)?;
            let mut j = serde_json::from_str::<Value>(&j)?;

            match &mut j {
                Value::Object(ref mut obj) => match obj.get(&key) {
                    Some(original_value) => {
                        // Assumption: the original value always has a correct type
                        let original_type = JsonType::from(original_value);
                        let new_value = original_type.parse(&value)?;

                        previous_value = obj.get(&key).map(|value| value.to_string());
                        obj.insert(
                            key.clone(),
                            new_value,
                        );
                        updated = true;

                        write_bytes(
                            &path,
                            &serde_json::to_vec_pretty(&j)?,
                            WriteMode::CreateOrTruncate,
                        )?;
                        break;
                    },
                    None => {
                        continue;
                    },
                },
                _ => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: (&j).into(),
                    });
                },
            }
        }

        if !updated {
            return Err(Error::InvalidConfigKey(key));
        }

        self.build_config = serde_json::from_str::<BuildConfig>(
            &read_string(&self.get_build_config_path()?)?,
        )?;
        self.query_config = serde_json::from_str::<QueryConfig>(
            &read_string(&self.get_query_config_path()?)?,
        )?;
        self.api_config = serde_json::from_str::<ApiConfig>(
            &read_string(&self.get_api_config_path()?)?,
        )?;

        Ok(previous_value)
    }
}
