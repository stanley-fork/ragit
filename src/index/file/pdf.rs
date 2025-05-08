use super::{AtomicToken, FileReaderImpl, Image};
use crate::chunk::ChunkExtraInfo;
use crate::error::Error;
use crate::index::BuildConfig;
use crate::uid::Uid;
use mupdf::{Colorspace, Document, ImageFormat, Matrix};
use ragit_pdl::ImageType;

// `PdfReader` has to be `Send`, but `mupdf::Document` is `!Send`.
// So it takes a bit inefficient route. It opens `Document`, converts
// 64 pages, and drops the `Document`. I chose the number 64 because
// 1. If it opens the `Document` per each page, that would be a performance
//    bottleneck.
// 2. If it opens the `Document` only once and loads all the pages, it'd use
//    too much memory if the pdf file is very large.
pub struct PdfReader {
    path: String,
    images: Vec<(AtomicToken, usize /* page_no */)>,
    page_count: usize,
    cursor: usize,
}

impl FileReaderImpl for PdfReader {
    fn new(path: &str, _config: &BuildConfig) -> Result<Self, Error> {
        let document = Document::open(path)?;
        let page_count = document.page_count()?.max(0) as usize;
        let mut result = PdfReader {
            images: vec![],
            path: path.to_string(),
            page_count,
            cursor: 0,
        };

        if result.cursor < result.page_count {
            for _ in 0..64 {
                result.images.push((convert_page(&document, result.cursor as i32)?, result.cursor + 1));
                result.cursor += 1;

                if result.cursor >= result.page_count {
                    break;
                }
            }
        }

        Ok(result)
    }

    fn load_tokens(&mut self) -> Result<(), Error> {
        if self.cursor < self.page_count {
            let document = Document::open(&self.path)?;

            for _ in 0..64 {
                self.images.push((convert_page(&document, self.cursor as i32)?, self.cursor + 1));
                self.cursor += 1;

                if self.cursor >= self.page_count {
                    break;
                }
            }
        }

        Ok(())
    }

    fn pop_all_tokens(&mut self) -> Result<Vec<AtomicToken>, Error> {
        let mut result = Vec::with_capacity(self.images.len());

        for (image, page_no) in self.images.iter() {
            result.push(image.clone());
            result.push(AtomicToken::PageBreak { extra_info: ChunkExtraInfo { page_no: Some(*page_no) } });
        }

        self.images = vec![];
        Ok(result)
    }

    fn has_more_to_read(&self) -> bool {
        self.cursor < self.page_count || !self.images.is_empty()
    }

    fn key(&self) -> String {
        String::from("pdf_reader_v1")
    }
}

fn convert_page(
    document: &Document,
    page: i32,
) -> Result<AtomicToken, Error> {
    let page = document.load_page(page)?;
    let bounds = page.bounds()?;
    let width = bounds.x1 - bounds.x0;
    let height = bounds.y1 - bounds.y0;
    let zoom = 1024.0 / width.max(height).max(0.1);

    let colorspace = Colorspace::device_rgb();
    let matrix = Matrix::new_scale(zoom, zoom);
    let mut pixmap = page.to_pixmap(&matrix, &colorspace, false, false)?;

    let (pixmap_width, pixmap_height) = pixmap.resolution();
    pixmap.set_resolution(
        (pixmap_width as f32 * zoom) as i32,
        (pixmap_height as f32 * zoom) as i32,
    );
    let mut bytes = vec![];
    pixmap.write_to(
        &mut bytes,
        ImageFormat::PNG,
    )?;
    let uid = Uid::new_image(&bytes);

    Ok(AtomicToken::Image(Image {
        bytes,
        image_type: ImageType::Png,
        uid,
    }))
}
