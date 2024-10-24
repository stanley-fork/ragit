use crate::error::Error;
use ragit_api::ImageType;

pub fn normalize_image(bytes: Vec<u8>, image_type: ImageType) -> Result<Vec<u8>, Error> {
    match image_type {
        ImageType::Png => Ok(bytes),  // TODO: maybe resize?
        _ => todo!(),  // TODO: convert to png
    }
}
