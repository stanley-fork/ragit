use super::Index;
use crate::error::Error;
use json::JsonValue;
use ragit_fs::read_string;
use std::collections::HashMap;

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
                _ => { /* TODO: Error? */ },
            }
        }

        Err(Error::InvalidConfigKey(key))
    }

    pub fn get_all(&self) -> Result<JsonValue, Error> {
        let mut result = HashMap::new();

        for path in [
            self.get_build_config_path()?,
            self.get_api_config_path()?,
            self.get_query_config_path()?,
        ] {
            let j = read_string(&path)?;
            let j = json::parse(&j)?;

            for (k, v) in j.entries() {
                let _d = result.insert(k.to_string(), v.clone()).is_none();
                debug_assert!(_d);
            }
        }

        Ok(result.into())
    }
}
