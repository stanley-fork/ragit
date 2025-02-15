use super::{AtomicToken, FileReaderImpl};
use crate::error::Error;
use crate::index::BuildConfig;
use crate::uid::Uid;
use ragit_fs::{extension, read_bytes, remove_file};
use ragit_pdl::ImageType;
use serde::{Deserialize, Serialize};
use std::fmt;

pub type Path = String;

#[derive(Clone, PartialEq)]
pub struct Image {
    pub uid: Uid,
    pub image_type: ImageType,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for Image {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.debug_struct("Image")
            .field("uid", &self.uid)
            .field("image_type", &self.image_type)
            .finish()
    }
}

pub fn normalize_image(bytes: Vec<u8>, image_type: ImageType) -> Result<Vec<u8>, Error> {
    let mut dynamic_image = image::load_from_memory_with_format(
        &bytes,
        image_type.into(),
    )?;

    if dynamic_image.width() > 1024 || dynamic_image.height() > 1024 {
        dynamic_image = dynamic_image.resize(1024, 1024, image::imageops::FilterType::Triangle);
    }

    // no modification at all
    else if image_type == ImageType::Png {
        return Ok(bytes);
    }

    // TODO: I don't want to save it to a tmp file. I want a direct `Vec<u8>`
    dynamic_image.save_with_format("._tmp.png", image::ImageFormat::Png)?;
    let bytes = read_bytes("._tmp.png")?;
    remove_file("._tmp.png")?;

    Ok(bytes)
}

pub struct ImageReader {
    path: Path,
    tokens: Vec<AtomicToken>,
    image_type: ImageType,
    is_exhausted: bool,
}

impl FileReaderImpl for ImageReader {
    fn new(path: &str, _config: &BuildConfig) -> Result<Self, Error> {
        Ok(ImageReader {
            path: path.to_string(),
            image_type: ImageType::from_extension(&extension(path)?.unwrap_or(String::new()))?,
            tokens: vec![],
            is_exhausted: false,
        })
    }

    fn load_tokens(&mut self) -> Result<(), Error> {
        if self.is_exhausted {
            Ok(())
        }

        else {
            let bytes = read_bytes(&self.path)?;
            let uid = Uid::new_image(&bytes);
            self.tokens.push(AtomicToken::Image(Image {
                bytes,
                image_type: self.image_type,
                uid,
            }));
            self.is_exhausted = true;
            Ok(())
        }
    }

    fn pop_all_tokens(&mut self) -> Result<Vec<AtomicToken>, Error> {
        let mut result = vec![];
        std::mem::swap(&mut self.tokens, &mut result);
        Ok(result)
    }

    fn has_more_to_read(&self) -> bool {
        !self.is_exhausted
    }

    fn key(&self) -> String {
        String::from("image_reader_v0")
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct ImageDescription {
    pub extracted_text: String,
    pub explanation: String,
}
