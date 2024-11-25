use crate::Error;
use crate::api_provider::ApiProvider;
use crate::json_type::JsonType;
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
                            got: key.into(),
                        }),
                    },
                    None => Err(Error::JsonObjectMissingField(api_provider.as_str().to_ascii_lowercase())),
                }
            } else {
                Err(Error::JsonTypeError {
                    expected: JsonType::Object,
                    got: (&j).into(),
                })
            },
            Err(e) => Err(Error::JsonError(e)),
        },
        Err(e) => Err(Error::FileError(e)),
    }
}
