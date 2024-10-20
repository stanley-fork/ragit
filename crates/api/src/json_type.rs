use crate::error::Error;
use json::JsonValue;

/// This enum is solely for error messages.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JsonType {
    Null,
    String,
    Number,
    Boolean,
    Object,
    Array,

    // for nicer error messages
    F64,
    F32,
    I64,
    U64,
    Usize,
}

impl JsonType {
    // NOTE: this function is used to parse input of `rag config --set`
    // NOTE: if `self` is `JsonType::Null`, then the actual type must be `Option<T>`, but there's no
    //       way we can guess what `T` is...
    // TODO: I don't like its implementation. It's too Javascript-ish.
    pub fn parse(&self, s: &str) -> Result<JsonValue, Error> {
        if s == "null" {
            return Ok(JsonValue::Null);
        }

        match self {
            JsonType::String => Ok(JsonValue::from(s)),
            JsonType::Number => match s.parse::<i64>() {
                Ok(n) => Ok(JsonValue::from(n)),
                _ => match s.parse::<f64>() {
                    Ok(n) => Ok(JsonValue::from(n)),
                    _ => match json::parse(s) {
                        Ok(v) => Err(Error::JsonTypeError {
                            expected: *self,
                            got: get_type(&v),
                        }),
                        Err(e) => Err(e.into()),
                    },
                },
            },
            JsonType::Boolean => match s {
                "true" => Ok(true.into()),
                "false" => Ok(false.into()),
                _ => match json::parse(s) {
                    Ok(v) => Err(Error::JsonTypeError {
                        expected: *self,
                        got: get_type(&v),
                    }),
                    Err(e) => Err(e.into()),
                },
            },
            // the most Javascript-ish part of this function. read the comments above
            JsonType::Null => match json::parse(s) {
                Ok(v) => Ok(v),
                Err(_) => Ok(JsonValue::from(s)),
            },
            _ => todo!(),
        }
    }
}

pub fn get_type(j: &JsonValue) -> JsonType {
    match j {
        JsonValue::Null => JsonType::Null,
        JsonValue::Short(_)
        | JsonValue::String(_) => JsonType::String,
        JsonValue::Number(_) => JsonType::Number,
        JsonValue::Boolean(_) => JsonType::Boolean,
        JsonValue::Object(_) => JsonType::Object,
        JsonValue::Array(_) => JsonType::Array,
    }
}
