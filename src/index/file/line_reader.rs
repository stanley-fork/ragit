use super::{AtomicToken, Config, FileReaderImpl};
use crate::error::Error;
use ragit_fs::FileError;
use std::io::{BufRead, BufReader};

pub struct LineReader {
    lines: BufReader<std::fs::File>,
    tokens: Vec<AtomicToken>,
    is_exhausted: bool,

    // The first N lines of the file is "headers". All the chunks start with "headers".
    // It's very useful for csv files: the first line of a csv file is usually the name of the columns.
    // By starting all the chunks with the column names, it makes each chunk look like a complete csv file.
    headers: Vec<AtomicToken>,
    header_length: usize,
}

impl FileReaderImpl for LineReader {
    fn new(path: &str, _config: &Config) -> Result<Self, Error> {
        match std::fs::File::open(path) {
            Ok(f) => Ok(LineReader {
                lines: BufReader::new(f),
                tokens: vec![],
                is_exhausted: false,
                headers: vec![],
                header_length: 0,
            }),
            Err(e) => Err(FileError::from_std(e, path).into()),
        }
    }

    fn load_tokens(&mut self) -> Result<(), Error> {
        if self.is_exhausted {
            return Ok(());
        }

        loop {
            // NOTE: `line` includes a newline character
            let mut line = String::new();

            if self.lines.read_line(&mut line)? == 0 {
                self.is_exhausted = true;
                break;
            }

            let curr_line = AtomicToken::String {
                char_len: line.len(),
                data: line,
            };

            if self.headers.len() < self.header_length {
                self.headers.push(curr_line);
            }

            else {
                self.tokens.push(curr_line);
                break;
            }
        }

        Ok(())
    }

    fn pop_all_tokens(&mut self) -> Result<Vec<AtomicToken>, Error> {
        let mut result = vec![];
        std::mem::swap(&mut self.tokens, &mut result);
        Ok(result)
    }

    fn chunk_header(&self) -> Vec<AtomicToken> {
        self.headers.clone()
    }

    fn has_more_to_read(&self) -> bool {
        !self.is_exhausted
    }

    fn key(&self) -> String {
        format!("line_reader_v0_{}", self.header_length)
    }
}

impl LineReader {
    pub fn set_header_length(mut self, header_length: usize) -> Self {
        self.header_length = header_length;
        self
    }
}
