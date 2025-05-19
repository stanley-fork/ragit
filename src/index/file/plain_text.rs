use super::{AtomicToken, FileReaderImpl};
use crate::error::Error;
use crate::index::BuildConfig;
use ragit_fs::FileError;
use std::fs::File;
use std::io::{Bytes, Read};

pub struct PlainTextReader {
    bytes: Bytes<File>,
    tokens: Vec<AtomicToken>,
    is_exhausted: bool,
    strict_mode: bool,
}

impl FileReaderImpl for PlainTextReader {
    fn new(path: &str, _root_dir: &str, config: &BuildConfig) -> Result<Self, Error> {
        match File::open(path) {
            Ok(f) => Ok(PlainTextReader {
                bytes: f.bytes(),
                tokens: vec![],
                is_exhausted: false,
                strict_mode: config.strict_file_reader,
            }),
            Err(e) => Err(FileError::from_std(e, path).into()),
        }
    }

    fn load_tokens(&mut self) -> Result<(), Error> {
        if self.is_exhausted {
            return Ok(());
        }

        // it is NOT a tokenizer
        // I just want to make sure that it does not split a word
        // into two different chunks
        let mut tmp_buffer = Vec::with_capacity(256);

        // it reads 4 MB at a time
        for _ in 0..(1 << 22) {
            if let Some(byte) = self.bytes.next() {
                let byte = byte?;

                if tmp_buffer.len() > 200 && (byte < 128 || byte >= 192)  // avoid utf-8 error
                    || tmp_buffer.len() >= 256  // in case there's no whitespace at all
                    || byte.is_ascii_whitespace() {

                    // I have decided to reject non utf-8 strings instead of using `String::from_utf8_lossy` because
                    //
                    // 1. Now that ragit continues processing files even if there's an erroneous file, it's okay to
                    //    throw more errors. It'll not bother the users.
                    // 2. Plain text reader is the default file reader. If a user mistakenly adds a random file, which
                    //    is likely to be a binary file, ragit will use the plain text reader. If it's using
                    //    `String::from_utf8_lossy`, it'll generate a chunk with tons of REPLACEMENT_CHARACTERs, which
                    //    is total waste of time and energy.
                    let s = String::from_utf8(tmp_buffer)?;
                    self.tokens.push(AtomicToken::String {
                        char_len: s.chars().count(),
                        data: s,
                    });
                    tmp_buffer = Vec::with_capacity(256);
                }

                tmp_buffer.push(byte);
            }

            else {
                self.is_exhausted = true;
                break;
            }
        }

        if !tmp_buffer.is_empty() {
            let s = if self.strict_mode {
                String::from_utf8(tmp_buffer)?
            } else {
                String::from_utf8_lossy(&tmp_buffer).to_string()
            };

            self.tokens.push(AtomicToken::String {
                char_len: s.chars().count(),
                data: s,
            });
        }

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
        String::from("plain_text_reader_v0")
    }
}
