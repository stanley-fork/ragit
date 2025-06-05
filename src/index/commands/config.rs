use super::{BuildConfig, Index};
use crate::{ApiConfig, QueryConfig};
use crate::error::Error;
use lazy_static::lazy_static;
use ragit_api::JsonType;
use ragit_fs::{WriteMode, read_string, write_bytes, write_string};
use serde_json::Value;
use std::collections::HashMap;

// This is a design mistake. I should have put all the configs in a single file.
#[allow(dead_code)]
enum ConfigType {
    Build,
    Query,
    Api,
}

lazy_static! {
    // These keys are added to ragit after the release of v0.1.1, so old knowledge-bases
    // might not have these keys in their config files. `rag config` command takes special
    // care to these keys.
    static ref NEWLY_ADDED_CONFIGS: HashMap<String, (Value, ConfigType)> = vec![
        ("super_rerank", (Value::Bool(false), ConfigType::Query)),
        ("enable_rag", (Value::Bool(true), ConfigType::Query)),
    ].into_iter().map(
        |(key, value)| (key.to_string(), value)
    ).collect();

    static ref DEPRECATED_CONFIGS: HashMap<String, String> = vec![
        ("max_titles", "Please use `max_summaries` and `max_retrieval`."),
        ("api_key", "Please set an environment variable or edit `models.json`."),
    ].into_iter().map(
        |(key, value)| (key.to_string(), value.to_string())
    ).collect();
}

impl Index {
    pub fn get_config_by_key(&self, key: String) -> Result<Value, Error> {
        if let Some(message) = DEPRECATED_CONFIGS.get(&key) {
            return Err(Error::DeprecatedConfig {
                key,
                message: message.to_string(),
            });
        }

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

        if let Some((value, _)) = NEWLY_ADDED_CONFIGS.get(&key) {
            Ok(value.clone())
        }

        else {
            Err(Error::InvalidConfigKey(key))
        }
    }

    /// It returns `Vec` instead of `HashMap` or `Json` since `Vec` is easier to sort by key.
    /// It does not sort the keys. It's your responsibility to do that.
    pub fn get_all_configs(&self) -> Result<Vec<(String, Value)>, Error> {
        let mut result = HashMap::new();

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
                        if !DEPRECATED_CONFIGS.contains_key(k) {
                            result.insert(k.to_string(), v.clone());
                        }
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

        for (k, (v, _)) in NEWLY_ADDED_CONFIGS.iter() {
            if !result.contains_key(k) {
                result.insert(k.to_string(), v.clone());
            }
        }

        Ok(result.into_iter().collect())
    }

    /// It returns the previous value, if exists.
    pub fn set_config_by_key(&mut self, key: String, value: String) -> Result<Option<String>, Error> {
        if let Some(message) = DEPRECATED_CONFIGS.get(&key) {
            return Err(Error::DeprecatedConfig {
                key,
                message: message.to_string(),
            });
        }

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
                Value::Object(obj) => match obj.get(&key) {
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
            if let Some((default_value, config)) = NEWLY_ADDED_CONFIGS.get(&key) {
                let config_path = match config {
                    ConfigType::Build => self.get_build_config_path()?,
                    ConfigType::Query => self.get_query_config_path()?,
                    ConfigType::Api => self.get_api_config_path()?,
                };
                let j = read_string(&config_path)?;
                let mut j = serde_json::from_str::<Value>(&j)?;
                let Value::Object(obj) = &mut j else { unreachable!() };
                obj.insert(key.clone(), JsonType::from(default_value).parse(&value)?);
                write_bytes(
                    &config_path,
                    &serde_json::to_vec_pretty(&j)?,
                    WriteMode::CreateOrTruncate,
                )?;
            }

            else {
                return Err(Error::InvalidConfigKey(key));
            }
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
