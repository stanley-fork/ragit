use super::{AtomicToken, FileReaderImpl};
use crate::error::Error;
use crate::index::BuildConfig;
use ragit_fs::FileError;
use std::fs::File;

// It uses a simple heuristic: it converts csv into jsonl so that
// each chunk contains more context.
pub struct CsvReader {
    iterator: csv::ByteRecordsIntoIter<File>,
    headers: Vec<String>,
    rows: Vec<AtomicToken>,  // an `AtomicToken` per row
    strict_mode: bool,
    is_exhausted: bool,
}

impl FileReaderImpl for CsvReader {
    fn new(path: &str, _root_dir: &str, config: &BuildConfig) -> Result<Self, Error> {
        match File::open(path) {
            Ok(f) => {
                let mut reader = csv::ReaderBuilder::new()
                    .delimiter(b',')
                    .flexible(!config.strict_file_reader)
                    .from_reader(f);
                let mut headers = vec![];

                for header in reader.byte_headers()? {
                    match String::from_utf8(header.to_vec()) {
                        Ok(s) => { headers.push(s); },
                        Err(e) => if config.strict_file_reader {
                            return Err(e.into());
                        } else {
                            headers.push(String::from_utf8_lossy(header).to_string());
                        },
                    }
                }

                Ok(CsvReader {
                    iterator: reader.into_byte_records(),
                    headers,
                    rows: vec![],
                    strict_mode: config.strict_file_reader,
                    is_exhausted: false,
                })
            },
            Err(e) => Err(FileError::from_std(e, path).into()),
        }
    }

    fn load_tokens(&mut self) -> Result<(), Error> {
        if self.is_exhausted {
            return Ok(());
        }

        match self.iterator.next() {
            Some(Ok(records)) => {
                let mut string_records = Vec::with_capacity(self.headers.len());

                for (index, record) in records.iter().enumerate() {
                    if index >= self.headers.len() {
                        break;
                    }

                    match String::from_utf8(record.to_vec()) {
                        Ok(s) => { string_records.push(s); },
                        Err(e) => if self.strict_mode {
                            return Err(e.into());
                        } else {
                            string_records.push(String::from_utf8_lossy(record).to_string());
                        },
                    }
                }

                let mut cells = Vec::with_capacity(self.headers.len());

                for (record, header) in string_records.iter().zip(self.headers.iter()) {
                    // heuristic: if `record` is numeric, it'd better strip off quotes
                    let record = match record.parse::<i64>() {
                        Ok(n) => n.to_string(),
                        _ => match record.parse::<f64>() {
                            Ok(f) => f.to_string(),
                            _ => format!("{record:?}"),
                        },
                    };

                    cells.push(format!("{header:?}: {record}"));
                }

                let row = format!("{}{}{}\n", "{", cells.join(", "), "}");
                self.rows.push(AtomicToken::String {
                    char_len: row.chars().count(),
                    data: row,
                });
            },
            Some(Err(e)) => if self.strict_mode {
                return Err(e.into());
            } else {
                // let's just skip this row
            },
            None => {
                self.is_exhausted = true;
            },
        }

        Ok(())
    }

    fn pop_all_tokens(&mut self) -> Result<Vec<AtomicToken>, Error> {
        let mut result = vec![];
        std::mem::swap(&mut self.rows, &mut result);
        Ok(result)
    }

    fn has_more_to_read(&self) -> bool {
        !self.is_exhausted || !self.rows.is_empty()
    }

    fn key(&self) -> String {
        String::from("csv_reader_v0")
    }
}
