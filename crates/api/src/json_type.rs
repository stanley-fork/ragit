use crate::error::Error;
use serde_json::Value;

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
    pub fn parse(&self, s: &str) -> Result<Value, Error> {
        if s == "null" {
            return Ok(Value::Null);
        }

        match self {
            JsonType::String => Ok(Value::from(s)),
            JsonType::Number => match s.parse::<i64>() {
                Ok(n) => Ok(Value::from(n)),
                _ => match s.parse::<f64>() {
                    Ok(n) => Ok(Value::from(n)),
                    _ => match serde_json::from_str::<Value>(s) {
                        Ok(v) => Err(Error::JsonTypeError {
                            expected: *self,
                            got: (&v).into(),
                        }),
                        Err(e) => Err(e.into()),
                    },
                },
            },
            JsonType::Boolean => match s {
                "true" => Ok(true.into()),
                "false" => Ok(false.into()),
                _ => match serde_json::from_str::<Value>(s) {
                    Ok(v) => Err(Error::JsonTypeError {
                        expected: *self,
                        got: (&v).into(),
                    }),
                    Err(e) => Err(e.into()),
                },
            },
            // the most Javascript-ish part of this function. read the comments above
            JsonType::Null => match serde_json::from_str(s) {
                Ok(v) => Ok(v),
                Err(_) => Ok(Value::from(s)),
            },
            _ => todo!(),
        }
    }
}

impl From<&Value> for JsonType {
    fn from(v: &Value) -> Self {
        match v {
            Value::Null => JsonType::Null,
            Value::String(_) => JsonType::String,
            Value::Number(_) => JsonType::Number,
            Value::Bool(_) => JsonType::Boolean,
            Value::Object(_) => JsonType::Object,
            Value::Array(_) => JsonType::Array,
        }
    }
}
