use ragit_fs::FileError;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    FileError(FileError),
    JsonSerdeError(serde_json::Error),
    WarpError(warp::Error),
    NoSuchSession(u128),
    ServerBusy,
}

impl From<FileError> for Error {
    fn from(e: FileError) -> Self {
        Error::FileError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::JsonSerdeError(e)
    }
}

impl From<warp::Error> for Error {
    fn from(e: warp::Error) -> Error {
        Error::WarpError(e)
    }
}
