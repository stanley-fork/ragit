use crate::error::Error;
use crate::uid::Uid;
use serde::Serialize;
use serde_json::Value;

/// Sometimes internal representations are not very readable. For example,
/// ragit internally represents a uid with 2 integers, but users expect a
/// hexadecimal string. This trait removes those quirks and makes it readable.
pub trait Prettify where Self: Serialize {
    fn prettify(&self) -> Result<Value, Error>;
}

impl<T: Prettify> Prettify for Vec<T> {
    fn prettify(&self) -> Result<Value, Error> {
        let mut result = Vec::with_capacity(self.len());

        for v in self.iter() {
            result.push(v.prettify()?)
        }

        Ok(Value::Array(result))
    }
}

pub(crate) fn prettify_uid(uid: &Value) -> Value {
    match uid {
        Value::Object(uid_map) => match (uid_map.get("high"), uid_map.get("low")) {
            (
                Some(Value::Number(high)),
                Some(Value::Number(low)),
            ) => match (high.as_u128(), low.as_u128()) {
                (Some(high), Some(low)) => {
                    let uid = Uid { high, low };
                    uid.to_string().into()
                },
                _ => uid.clone(),
            },
            _ => uid.clone(),
        },
        _ => uid.clone(),
    }
}

pub(crate) fn prettify_timestamp(timestamp: &Value) -> Value {
    match timestamp {
        Value::Number(timestamp_n) => match timestamp_n.as_i64() {
            Some(timestamp) => chrono::DateTime::from_timestamp(timestamp, 0).map(
                |d| d.to_rfc3339()
            ).unwrap_or_else(|| String::from("error")).into(),
            None => timestamp.clone(),
        },
        _ => timestamp.clone(),
    }
}
