use super::{Config, Index};
use crate::{ApiConfigRaw, QueryConfig};
use crate::error::Error;
use json::JsonValue;
use ragit_api::{JsonType, get_type};
use ragit_fs::{WriteMode, read_string, write_string};
use std::collections::HashMap;

// TODO: refactor names
//       { meta_get, meta_get_all, meta_set, meta_remove, meta_remove_all }
//       { get_config_by_key, get_all, set_config_by_key }
//       -> 2 different set of names for similar features
impl Index {
    pub fn get_config_by_key(&self, key: String) -> Result<JsonValue, Error> {
        for path in [
            self.get_build_config_path()?,
            self.get_api_config_path()?,
            self.get_query_config_path()?,
        ] {
            let j = read_string(&path)?;
            let j = json::parse(&j)?;

            match j {
                JsonValue::Object(obj) => match obj.get(&key) {
                    Some(v) => { return Ok(v.clone()) },
                    _ => {},
                },
                _ => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::Object,
                        got: get_type(&j),
                    });
                },
            }
        }

        Err(Error::InvalidConfigKey(key))
    }

    /// It returns `Vec` instead of `HashMap` or `Json` since `Vec` is easier to sort by key.
    /// It does not sort the keys. It's your responsibility to do that.
    pub fn get_all(&self) -> Result<Vec<(String, JsonValue)>, Error> {
        let mut result = vec![];

        for path in [
            self.get_build_config_path()?,
            self.get_api_config_path()?,
            self.get_query_config_path()?,
        ] {
            let j = read_string(&path)?;
            let j = json::parse(&j)?;

            for (k, v) in j.entries() {
                result.push((k.to_string(), v.clone()));
            }
        }

        Ok(result)
    }
    pub fn set_config_by_key(&mut self, key: String, value: String) -> Result<Option<String>, Error> {  // returns the previous value, if exists
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
            let mut j = json::parse(&j)?;

            match &mut j {
                JsonValue::Object(ref mut obj) => match obj.get(&key) {
                    Some(original_value) => {
                        // Assumption: the original value always has a correct type
                        let original_type = get_type(original_value);
                        let new_value = original_type.parse(&value)?;

                        previous_value = obj.get(&key).map(|value| value.dump());
                        obj.insert(
                            &key,
                            new_value,
                        );
                        updated = true;

                        write_string(
                            &path,
                            &j.pretty(4),
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
                        got: get_type(&j),
                    });
                },
            }
        }

        if !updated {
            return Err(Error::InvalidConfigKey(key));
        }

        self.config = serde_json::from_str::<Config>(
            &read_string(&self.get_build_config_path()?)?,
        )?;
        self.query_config = serde_json::from_str::<QueryConfig>(
            &read_string(&self.get_query_config_path()?)?,
        )?;
        self.api_config_raw = serde_json::from_str::<ApiConfigRaw>(
            &read_string(&self.get_api_config_path()?)?,
        )?;
        self.api_config = self.init_api_config(&self.api_config_raw)?;

        Ok(previous_value)
    }
}
