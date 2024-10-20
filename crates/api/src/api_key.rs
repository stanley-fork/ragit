use crate::Error;
use crate::api_provider::ApiProvider;
use crate::json_type::{JsonType, get_type};
use json::JsonValue;
use ragit_fs::read_string;

// for testing purpose
pub fn load_api_key(path: &str, api_provider: ApiProvider) -> Result<String, Error> {
    match read_string(path) {
        Ok(j) => match json::parse(&j) {
            Ok(j) => if let JsonValue::Object(object) = j {
                match object.get(&api_provider.as_str().to_ascii_lowercase()) {
                    Some(key) => match key.as_str() {
                        Some(s) => Ok(s.to_string()),
                        None => Err(Error::JsonTypeError {
                            expected: JsonType::String,
                            got: get_type(key),
                        }),
                    },
                    None => Err(Error::JsonObjectMissingField(api_provider.as_str().to_ascii_lowercase())),
                }
            } else {
                Err(Error::JsonTypeError {
                    expected: JsonType::Object,
                    got: get_type(&j),
                })
            },
            Err(e) => Err(Error::JsonError(e)),
        },
        Err(e) => Err(Error::FileError(e)),
    }
}
