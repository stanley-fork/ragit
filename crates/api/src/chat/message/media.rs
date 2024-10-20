use super::MessageContent;
use crate::Error;
use ragit_fs::{extension, read_bytes};
use regex::Regex;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum ImageType {
    Jpeg,
    Png,
    Gif,
    Webp,
}

impl ImageType {
    // for anthropic api
    pub fn get_media_type(&self) -> &str {
        match self {
            ImageType::Jpeg => "image/jpeg",
            ImageType::Png => "image/png",
            ImageType::Gif => "image/gif",
            ImageType::Webp => "image/webp",
        }
    }

    pub fn from_media_type(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase() {
            s if s == "image/jpeg" || s == "image/jpg" => Some(ImageType::Jpeg),
            s if s == "image/png" => Some(ImageType::Png),
            s if s == "image/gif" => Some(ImageType::Gif),
            s if s == "image/webp" => Some(ImageType::Webp),
            _ => None,
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase() {
            ext if ext == "png" => Some(ImageType::Png),
            ext if ext == "jpeg" || ext == "jpg" => Some(ImageType::Jpeg),
            ext if ext == "gif" => Some(ImageType::Gif),
            ext if ext == "webp" => Some(ImageType::Webp),
            _ => None,
        }
    }

    pub fn infer_from_path(path: &str) -> Option<Self> {
        let ext_re = Regex::new(r".+\.([^.]+)$").unwrap();

        if let Some(ext) = ext_re.captures(path) {
            ImageType::from_extension(ext.get(1).unwrap().as_str())
        }

        else {
            None
        }
    }

    pub fn to_extension(&self) -> &str {
        match self {
            ImageType::Jpeg => "jpg",
            ImageType::Png => "png",
            ImageType::Gif => "gif",
            ImageType::Webp => "webp",
        }
    }
}

pub struct MediaMessageBuilder {
    /// It infers the media-type from file extensions.
    pub paths: Vec<String>,

    /// prompt that follows the images
    pub prompt: Option<String>,
}

impl MediaMessageBuilder {
    pub fn build(
        &self,
    ) -> Result<Vec<MessageContent>, Error> {
        let mut content = vec![];

        for path in self.paths.iter() {
            match extension(path)? {
                Some(ext) => match ext.to_ascii_lowercase() {
                    ext if ImageType::from_extension(&ext).is_some() => {
                        let bytes = read_bytes(path)?;

                        // TODO: auto-resize

                        content.push(MessageContent::Image {
                            image_type: ImageType::from_extension(&ext).unwrap(),
                            bytes,
                        });
                    },
                    ext if ext == "pdf" => todo!(),
                    ext => {
                        return Err(Error::UnsupportedMediaFormat { extension: Some(ext.to_string()) });
                    },
                },
                None => {
                    return Err(Error::UnsupportedMediaFormat { extension: None });
                },
            }
        }

        if let Some(prompt) = &self.prompt {
            content.push(MessageContent::String(prompt.to_string()));
        }

        Ok(content)
    }
}
