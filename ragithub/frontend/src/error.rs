#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    RequestFailure { status: u16 },
    ReqwestError(reqwest::Error),
    WarpError(warp::Error),
    GrassError(grass::Error),
    TeraError(tera::Error),
    FileError(ragit_fs::FileError),
    JsonSerdeError(serde_json::Error),
}

impl From<warp::Error> for Error {
    fn from(e: warp::Error) -> Self {
        Error::WarpError(e)
    }
}

impl From<grass::Error> for Error {
    fn from(e: grass::Error) -> Self {
        Error::GrassError(e)
    }
}

impl From<tera::Error> for Error {
    fn from(e: tera::Error) -> Self {
        Error::TeraError(e)
    }
}

impl From<Box<grass::Error>> for Error {
    fn from(e: Box<grass::Error>) -> Self {
        Error::GrassError(e.as_ref().clone())
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::ReqwestError(e)
    }
}

impl From<ragit_fs::FileError> for Error {
    fn from(e: ragit_fs::FileError) -> Self {
        Error::FileError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::JsonSerdeError(e)
    }
}
