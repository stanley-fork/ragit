use super::{AtomicToken, BuildConfig, FileReaderImpl};
use crate::error::Error;
use ragit_fs::FileError;
use std::io::{BufRead, BufReader};
use std::fs::File;

pub struct LineReader {
    lines: BufReader<File>,
    tokens: Vec<AtomicToken>,
    is_exhausted: bool,
}

impl FileReaderImpl for LineReader {
    fn new(path: &str, _config: &BuildConfig) -> Result<Self, Error> {
        match File::open(path) {
            Ok(f) => Ok(LineReader {
                lines: BufReader::new(f),
                tokens: vec![],
                is_exhausted: false,
            }),
            Err(e) => Err(FileError::from_std(e, path).into()),
        }
    }

    fn load_tokens(&mut self) -> Result<(), Error> {
        if self.is_exhausted {
            return Ok(());
        }

        // NOTE: `line` includes a newline character
        let mut line = String::new();

        if self.lines.read_line(&mut line)? == 0 {
            self.is_exhausted = true;
            return Ok(());
        }

        let curr_line = AtomicToken::String {
            char_len: line.chars().count(),
            data: line,
        };

        self.tokens.push(curr_line);
        Ok(())
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
        String::from("line_reader_v1")
    }
}
