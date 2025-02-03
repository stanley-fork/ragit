use super::{Prettify, prettify_timestamp, prettify_uid};
use crate::chunk::Chunk;
use crate::error::Error;
use serde_json::Value;

pub type ChunkSchema = Chunk;

impl Prettify for ChunkSchema {
    fn prettify(&self) -> Result<Value, Error> {
        let mut result = serde_json::to_value(self)?;

        if let Value::Object(obj) = &mut result {
            match obj.get_mut("images") {
                Some(Value::Array(images)) => {
                    for image in images.iter_mut() {
                        *image = prettify_uid(image);
                    }
                },
                _ => {},
            }

            match obj.get_mut("uid") {
                Some(uid) => { *uid = prettify_uid(uid); },
                None => {},
            }

            match obj.get_mut("timestamp") {
                Some(timestamp) => { *timestamp = prettify_timestamp(timestamp); },
                None => {},
            }
        }

        Ok(result)
    }
}
