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

    pub fn from_media_type(s: &str) -> Result<Self, Error> {
        match s.to_ascii_lowercase() {
            s if s == "image/jpeg" || s == "image/jpg" => Ok(ImageType::Jpeg),
            s if s == "image/png" => Ok(ImageType::Png),
            s if s == "image/gif" => Ok(ImageType::Gif),
            s if s == "image/webp" => Ok(ImageType::Webp),
            _ => Err(Error::InvalidImageType(s.to_string())),
        }
    }

    pub fn from_extension(ext: &str) -> Result<Self, Error> {
        match ext.to_ascii_lowercase() {
            ext if ext == "png" => Ok(ImageType::Png),
            ext if ext == "jpeg" || ext == "jpg" => Ok(ImageType::Jpeg),
            ext if ext == "gif" => Ok(ImageType::Gif),
            ext if ext == "webp" => Ok(ImageType::Webp),
            _ => Err(Error::InvalidImageType(ext.to_string())),
        }
    }

    pub fn infer_from_path(path: &str) -> Result<Self, Error> {
        let ext_re = Regex::new(r".+\.([^.]+)$").unwrap();

        if let Some(ext) = ext_re.captures(path) {
            ImageType::from_extension(ext.get(1).unwrap().as_str())
        }

        else {
            Err(Error::InvalidImageType(path.to_string()))
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
                    ext if ImageType::from_extension(&ext).is_ok() => {
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

impl From<ImageType> for image::ImageFormat {
    fn from(image_type: ImageType) -> Self {
        match image_type {
            ImageType::Jpeg => image::ImageFormat::Jpeg,
            ImageType::Png => image::ImageFormat::Png,
            ImageType::Gif => image::ImageFormat::Gif,
            ImageType::Webp => image::ImageFormat::WebP,
        }
    }
}
