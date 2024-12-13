use crate::json_type::JsonType;
use ragit_fs::FileError;

#[derive(Debug)]
pub enum Error {
    JsonTypeError {
        expected: JsonType,
        got: JsonType,
    },
    JsonObjectInvalidField(String),
    JsonObjectMissingField(String),
    InvalidModelKind(String),
    PdlError(ragit_pdl::Error),
    FileError(FileError),

    /// If you see this error, there must be a bug in this library
    NoTry,

    /// see <https://docs.rs/reqwest/latest/reqwest/struct.Error.html>
    ReqwestError(reqwest::Error),

    /// see <https://docs.rs/json/latest/json/enum.Error.html>
    JsonError(json::Error),

    /// see <https://docs.rs/serde_json/latest/serde_json/struct.Error.html>
    JsonSerdeError(serde_json::Error),

    /// see <https://docs.rs/tera/latest/tera/struct.Error.html>
    TeraError(tera::Error),

    WrongSchema(String),
    ServerError {
        status_code: u16,
        body: Result<String, reqwest::Error>,
    },
    UnsupportedMediaFormat {
        extension: Option<String>,
    },
}

impl From<ragit_pdl::Error> for Error {
    fn from(e: ragit_pdl::Error) -> Error {
        Error::PdlError(e)
    }
}

impl From<FileError> for Error {
    fn from(e: FileError) -> Error {
        Error::FileError(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::ReqwestError(e)
    }
}

impl From<json::Error> for Error {
    fn from(e: json::Error) -> Error {
        Error::JsonError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::JsonSerdeError(e)
    }
}

impl From<tera::Error> for Error {
    fn from(e: tera::Error) -> Error {
        Error::TeraError(e)
    }
}
