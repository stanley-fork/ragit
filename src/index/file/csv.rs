use super::{AtomicToken, FileReaderImpl};
use crate::error::Error;
use crate::index::Config;

/*
# TODO
import csv

with open("test.csv", "w") as f:
    writer = csv.writer(f)
    writer.writerow(["a", "b", "c"])
    writer.writerow(["a", "b b", "c"])
    writer.writerow(["a", "b\nb", "c"])
    writer.writerow(["a", "b,b", "c"])
    writer.writerow(["a", "b\"b", "c"])
    writer.writerow(["a", "b'b", "c"])
    writer.writerow(["a", "b'\"b", "c"])
    writer.writerow(["a", "\"bb", "c"])

'''
a,b,c
a,b b,c
a,"b
b",c
a,"b,b",c
a,"b""b",c
a,b'b,c
a,"b'""b",c
a,"""bb",c

'''
*/
pub struct CsvReader {}

impl FileReaderImpl for CsvReader {
    fn new(path: &str, config: &Config) -> Result<Self, Error> {
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
