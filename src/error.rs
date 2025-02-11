use crate::index::IIStatus;
use crate::uid::Uid;
pub use ragit_api::{Error as ApiError, JsonType};
use ragit_fs::FileError;
use std::string::FromUtf8Error;

pub type Path = String;

#[derive(Debug)]
pub enum Error {
    JsonTypeError {
        expected: JsonType,
        got: JsonType,
    },
    IndexAlreadyExists(Path),
    InvalidConfigKey(String),
    InvalidImageType(String),
    InvalidUid(String),
    PromptMissing(String),
    IndexNotFound,
    NoSuchChunk(Uid),
    NoSuchFile { path: Option<Path>, uid: Option<Uid> },
    NoSuchMeta(String),
    CorruptedFile { path: Path, message: Option<String> },
    CliError {
        message: String,
        span: (String, usize, usize),  // (args, error_from, error_to)
    },
    UidQueryError(String),
    BrokenHash(String),
    BrokenPrompt(String),
    CloneRequestError {
        code: Option<u16>,
        url: String,
    },
    InvalidVersionString(String),
    CannotMigrate {
        from: String,
        to: String,
    },
    CannotClone(String),
    CannotUpdateII(IIStatus),
    CannotAddFile {
        file: String,  // rel_path
        message: String,
    },
    MergeConflict(Uid),
    MPSCError(String),
    CannotDeserializeUid,

    /// If a user sees this error, that's a bug in ragit.
    Internal(String),

    // If you're implementing a new FileReaderImpl, and don't know which variant to use,
    // just use this one.
    FileReaderError(String),

    // TODO: more enum variants for this type?
    BrokenIndex(String),
    BrokenII(String),

    /// see <https://docs.rs/reqwest/latest/reqwest/struct.Error.html>
    ReqwestError(reqwest::Error),

    /// see <https://docs.rs/serde_json/latest/serde_json/struct.Error.html>
    JsonSerdeError(serde_json::Error),

    /// see <https://docs.rs/image/latest/image/error/enum.ImageError.html>
    ImageError(image::ImageError),

    /// see <https://docs.rs/csv/latest/csv/struct.Error.html>
    CsvError(csv::Error),

    /// see <https://docs.rs/url/latest/url/enum.ParseError.html>
    UrlParseError(url::ParseError),

    FileError(FileError),
    StdIoError(std::io::Error),
    Utf8Error(FromUtf8Error),

    // I'm too lazy to add all the variants of ragit_api::Error
    ApiError(ApiError),

    // I'm too lazy to add all the variants of ragit_pdl::Error
    PdlError(ragit_pdl::Error),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::ReqwestError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::JsonSerdeError(e)
    }
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Error {
        Error::ImageError(e)
    }
}

impl From<csv::Error> for Error {
    fn from(e: csv::Error) -> Error {
        Error::CsvError(e)
    }
}

impl From<FileError> for Error {
    fn from(e: FileError) -> Error {
        Error::FileError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::StdIoError(e)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Error {
        Error::Utf8Error(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Error {
        Error::UrlParseError(e)
    }
}

impl From<ApiError> for Error {
    fn from(e: ApiError) -> Self {
        match e {
            ApiError::JsonTypeError { expected, got } => Error::JsonTypeError { expected, got },
            ApiError::ReqwestError(e) => Error::ReqwestError(e),
            ApiError::JsonSerdeError(e) => Error::JsonSerdeError(e),
            ApiError::FileError(e) => Error::FileError(e),
            e => Error::ApiError(e),
        }
    }
}

impl From<ragit_pdl::Error> for Error {
    fn from(e: ragit_pdl::Error) -> Self {
        match e {
            ragit_pdl::Error::InvalidImageType(e) => Error::InvalidImageType(e),
            e => Error::PdlError(e),
        }
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
