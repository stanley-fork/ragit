use super::{AtomicToken, FileReaderImpl, Image};
use crate::error::Error;
use crate::index::BuildConfig;
use crate::uid::Uid;
use ragit_fs::{
    extension,
    read_bytes,
    read_dir,
};
use ragit_pdl::ImageType;

// This is very very experimental pdf reader.
// Its strategy is to 1) convert each page into images, 2) use image reader to build a knowledge-base.
// This strategy works well, but the problem is that there's no easy-to-use rust library that converts
// pdf page to images. For now, it uses a Python script to do that (see ./pdf.py).
//
// Let's say there's a pdf file: `sample.pdf`. It expects the python script to make a directory `sample.pdf-pages/`.
// The directory must contain the image files, and the file names must be sorted by their original order.
// Also, it expects each page to have exactly 3 images. It creates 1 chunk per 1 image.
pub struct PdfReader {
    images: Vec<AtomicToken>,
    pages: Vec<String>,  // path to images
    cursor: usize,
}

impl FileReaderImpl for PdfReader {
    fn new(path: &str, _config: &BuildConfig) -> Result<Self, Error> {
        let pages = read_dir(&format!("{path}-pages"), true)?;
        Ok(PdfReader {
            images: vec![],
            pages,
            cursor: 0,
        })
    }

    fn load_tokens(&mut self) -> Result<(), Error> {
        if self.cursor < self.pages.len() {
            let path = &self.pages[self.cursor];
            let bytes = read_bytes(path)?;
            let uid = Uid::new_image(&bytes);
            self.images.push(AtomicToken::Image(Image {
                bytes,
                image_type: ImageType::from_extension(&extension(path)?.unwrap_or(String::new()))?,
                uid,
            }));

            self.cursor += 1;
        }

        Ok(())
    }

    fn pop_all_tokens(&mut self) -> Result<Vec<AtomicToken>, Error> {
        let mut result = Vec::with_capacity(self.images.len());

        for image in self.images.iter() {
            result.push(image.clone());
            result.push(AtomicToken::Separator);
        }

        self.images = vec![];
        Ok(result)
    }

    fn has_more_to_read(&self) -> bool {
        self.cursor < self.pages.len() || !self.images.is_empty()
    }

    fn key(&self) -> String {
        String::from("pdf_reader_v0")
    }
}
