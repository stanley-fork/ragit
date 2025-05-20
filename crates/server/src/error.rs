use ragit_fs::FileError;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    FileError(FileError),
    JsonSerdeError(serde_json::Error),
    WarpError(warp::Error),
    NoSuchSession(String),
    NoSuchArchive(String),
    CliError {
        message: String,
        span: (String, usize, usize),  // (args, error_from, error_to)
    },
    SqlxError(sqlx::Error),
    RagitError(ragit::Error),
    InsecurePath(String),
    InvalidUtf8,
    ServerBusy,
    ConfigNotInitialized,
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

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Error {
        Error::SqlxError(e)
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

impl From<ragit::Error> for Error {
    fn from(e: ragit::Error) -> Self {
        match e {
            ragit::Error::FileError(e) => Error::FileError(e),
            ragit::Error::JsonSerdeError(e) => Error::JsonSerdeError(e),
            ragit::Error::CliError { message, span } => Error::CliError { message, span },
            e => Error::RagitError(e),
        }
    }
}
