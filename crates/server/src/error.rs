use ragit_fs::FileError;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    FileError(FileError),
    NoSuchSession(u128),
    ServerBusy,
}

impl From<FileError> for Error {
    fn from(e: FileError) -> Self {
        Error::FileError(e)
    }
}
