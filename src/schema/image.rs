use super::{Prettify, prettify_uid};
use crate::constant::IMAGE_DIR_NAME;
use crate::error::Error;
use crate::index::Index;
use crate::uid::Uid;
use ragit_api::JsonType;
use ragit_fs::{
    file_size,
    read_bytes,
    read_string,
    set_extension,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
pub struct ImageSchema {
    pub uid: Uid,
    pub extracted_text: String,
    pub explanation: String,
    pub size: u64,  // bytes

    /// For optimization, you can load `ImageSchema` without bytes.
    /// In such case, this field is an empty vector.
    pub bytes: Vec<u8>,
}

impl Index {
    pub fn get_image_schema(&self, uid: Uid, load_bytes: bool) -> Result<ImageSchema, Error> {
        let description_path = Index::get_uid_path(
            &self.root_dir,
            IMAGE_DIR_NAME,
            uid,
            Some("json"),
        )?;
        let image_path = set_extension(&description_path, "png")?;
        let description = read_string(&description_path)?;
        let description = serde_json::from_str::<Value>(&description)?;
        let bytes = if load_bytes {
            read_bytes(&image_path)?
        } else {
            vec![]
        };

        match description {
            Value::Object(obj) => match (obj.get("extracted_text"), obj.get("explanation")) {
                (Some(extracted_text), Some(explanation)) => Ok(ImageSchema {
                    uid,
                    extracted_text: extracted_text.to_string(),
                    explanation: explanation.to_string(),
                    size: file_size(&image_path)?,
                    bytes,
                }),
                _ => Err(Error::BrokenIndex(format!("`{description_path}` has a wrong schema."))),
            },
            _ => Err(Error::JsonTypeError {
                expected: JsonType::Object,
                got: (&description).into(),
            }),
        }
    }
}

impl Prettify for ImageSchema {
    fn prettify(&self) -> Result<Value, Error> {
        let mut result = serde_json::to_value(self)?;

        if let Value::Object(obj) = &mut result {
            match obj.get_mut("uid") {
                Some(uid) => { *uid = prettify_uid(uid); },
                None => {},
            }

            obj.remove("bytes");
        }

        Ok(result)
    }
}
