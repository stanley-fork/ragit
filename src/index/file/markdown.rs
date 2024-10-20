use super::{AtomicToken, FileReaderImpl};
use crate::error::Error;

pub struct MarkdownReader {}

impl FileReaderImpl for MarkdownReader {
    fn new(path: &str) -> Result<Self, Error> {
        todo!()
    }

    fn load_tokens(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn pop_all_tokens(&mut self) -> Result<Vec<AtomicToken>, Error> {
        todo!()
    }

    fn has_more_to_read(&self) -> bool {
        todo!()
    }

    fn key(&self) -> String {
        todo!()
    }
}
