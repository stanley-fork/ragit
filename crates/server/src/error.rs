use ragit_fs::FileError;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    FileError(FileError),
    JsonSerdeError(serde_json::Error),
    WarpError(warp::Error),
    NoSuchSession(u128),
    CliError {
        message: String,
        span: (String, usize, usize),  // (args, error_from, error_to)
    },
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

impl From<ragit_cli::Error> for Error {
    fn from(e: ragit_cli::Error) -> Self {
        Error::CliError {
            message: e.kind.render(),
            span: e.span.unwrap_rendered(),
        }
    }
}
