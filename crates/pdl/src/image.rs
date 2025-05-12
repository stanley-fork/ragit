use crate::error::Error;
use regex::Regex;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImageType {
    Jpeg,
    Png,
    Gif,
    Webp,
    Svg,
}

impl ImageType {
    // for anthropic api
    pub fn get_media_type(&self) -> &str {
        match self {
            ImageType::Jpeg => "image/jpeg",
            ImageType::Png => "image/png",
            ImageType::Gif => "image/gif",
            ImageType::Webp => "image/webp",

            // I'm not sure whether it'd work with anthropic api
            ImageType::Svg => "image/svg+xml",
        }
    }

    pub fn from_media_type(s: &str) -> Result<Self, Error> {
        match s.to_ascii_lowercase() {
            s if s == "image/jpeg" || s == "image/jpg" => Ok(ImageType::Jpeg),
            s if s == "image/png" => Ok(ImageType::Png),
            s if s == "image/gif" => Ok(ImageType::Gif),
            s if s == "image/webp" => Ok(ImageType::Webp),
            s if s == "image/svg+xml" => Ok(ImageType::Svg),
            _ => Err(Error::InvalidImageType(s.to_string())),
        }
    }

    pub fn from_extension(ext: &str) -> Result<Self, Error> {
        match ext.to_ascii_lowercase() {
            ext if ext == "png" => Ok(ImageType::Png),
            ext if ext == "jpeg" || ext == "jpg" => Ok(ImageType::Jpeg),
            ext if ext == "gif" => Ok(ImageType::Gif),
            ext if ext == "webp" => Ok(ImageType::Webp),
            ext if ext == "svg" => Ok(ImageType::Svg),
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
            ImageType::Svg => "svg",
        }
    }
}

impl TryFrom<ImageType> for image::ImageFormat {
    type Error = Error;

    fn try_from(image_type: ImageType) -> Result<Self, Error> {
        match image_type {
            ImageType::Jpeg => Ok(image::ImageFormat::Jpeg),
            ImageType::Png => Ok(image::ImageFormat::Png),
            ImageType::Gif => Ok(image::ImageFormat::Gif),
            ImageType::Webp => Ok(image::ImageFormat::WebP),
            ImageType::Svg => Err(Error::InvalidImageType(image_type.to_extension().to_string())),
        }
    }
}
