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
    CorruptedFile(Path),
    NotImplemented(String),
    CliError(String),  // TODO: spans?
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
    CannotUpdateII(IIStatus),

    // If you're implementing a new FileReaderImpl, and don't know which variant to use,
    // just use this one.
    FileReaderError(String),

    // TODO: more enum variants for this type?
    BrokenIndex(String),

    /// see <https://docs.rs/reqwest/latest/reqwest/struct.Error.html>
    ReqwestError(reqwest::Error),

    /// see <https://docs.rs/json/latest/json/enum.Error.html>
    JsonError(json::Error),

    /// see <https://docs.rs/serde_json/latest/serde_json/struct.Error.html>
    JsonSerdeError(serde_json::Error),

    /// see <https://docs.rs/image/latest/image/error/enum.ImageError.html>
    ImageError(image::ImageError),

    /// see <https://docs.rs/url/latest/url/enum.ParseError.html>
    UrlParseError(url::ParseError),

    FileError(FileError),
    StdIoError(std::io::Error),
    Utf8Error(FromUtf8Error),

    // I'm too lazy to add all the variants of ragit_api::Error
    ApiError(ApiError),
}

impl From<json::Error> for Error {
    fn from(e: json::Error) -> Error {
        Error::JsonError(e)
    }
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
            ApiError::JsonError(e) => Error::JsonError(e),
            ApiError::JsonSerdeError(e) => Error::JsonSerdeError(e),
            ApiError::FileError(e) => Error::FileError(e),
            ApiError::InvalidImageType(e) => Error::InvalidImageType(e),
            e => Error::ApiError(e),
        }
    }
}
