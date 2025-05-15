use crate::schema::SchemaParseError;
use ragit_fs::FileError;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum Error {
    RoleMissing,
    InvalidPdl(String),
    InvalidTurnSeparator(String),
    InvalidInlineBlock,
    InvalidImageType(String),
    InvalidRole(String),
    FileError(FileError),
    Utf8Error(FromUtf8Error),
    SchemaParseError(SchemaParseError),

    /// see <https://docs.rs/serde_json/latest/serde_json/struct.Error.html>
    JsonSerdeError(serde_json::Error),

    /// see <https://docs.rs/base64/latest/base64/enum.DecodeError.html>
    Base64DecodeError(base64::DecodeError),

    /// https://docs.rs/tera/latest/tera/struct.Error.html
    TeraError(tera::Error),
}

impl From<SchemaParseError> for Error {
    fn from(e: SchemaParseError) -> Error {
        match e {
            SchemaParseError::Utf8Error(e) => Error::Utf8Error(e),
            e => Error::SchemaParseError(e),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::JsonSerdeError(e)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Error {
        Error::Utf8Error(e)
    }
}

impl From<FileError> for Error {
    fn from(e: FileError) -> Error {
        Error::FileError(e)
    }
}

impl From<tera::Error> for Error {
    fn from(e: tera::Error) -> Error {
        Error::TeraError(e)
    }
}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Error {
        Error::Base64DecodeError(e)
    }
}
