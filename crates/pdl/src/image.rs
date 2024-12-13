use crate::error::Error;
use regex::Regex;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
