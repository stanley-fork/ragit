use crate::error::Error;
use ragit_api::ImageType;
use ragit_fs::{read_bytes, remove_file};

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

    // TODO: I don't want save it to a tmp file. I want a direct `Vec<u8>`
    dynamic_image.save_with_format("._tmp.png", image::ImageFormat::Png)?;
    let bytes = read_bytes("._tmp.png")?;
    remove_file("._tmp.png")?;

    Ok(bytes)
}
