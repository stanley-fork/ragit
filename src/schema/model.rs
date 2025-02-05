use super::Prettify;
use crate::error::Error;
use ragit_api::{Model, ModelRaw};
use serde_json::Value;

pub type ModelSchema = Model;

impl Prettify for ModelSchema {
    fn prettify(&self) -> Result<Value, Error> {
        let result = ModelRaw::from(self);
        let mut result = serde_json::to_value(result)?;

        if let Value::Object(obj) = &mut result {
            match obj.get_mut("api_key") {
                Some(Value::String(key)) => {
                    let chars_count = key.chars().count();

                    if chars_count > 8 {
                        *key = format!(
                            "{}{}",
                            key.chars().take(4).collect::<String>(),
                            "*".repeat(chars_count - 4),
                        );
                    }

                    else {
                        *key = String::from("*".repeat(chars_count));
                    }
                },
                _ => {},
            }

            for key in [
                "api_timeout",
                "explanation",
                "api_key",
                "api_env_var",
            ] {
                if let Some(Value::Null) = obj.get(key) {
                    obj.remove(key);
                }
            }
        }

        Ok(result)
    }
}
