use crate::uid::Uid;
pub use ragit_api::Error as ApiError;
pub use ragit_pdl::JsonType;
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
    NoFileToRemove,
    NoRemoteToPullFrom,
    NoSuchMeta(String),
    NoSummary,
    CorruptedFile { path: Path, message: Option<String> },
    CliError {
        message: String,
        span: (String, usize, usize),  // (args, error_from, error_to)
    },
    UidQueryError(String),
    BrokenHash(String),
    BrokenPrompt(String),
    BrokenArchive(String),
    RequestFailure {
        context: Option<String>,  // clone | push | pull ...
        code: Option<u16>,
        url: String,
    },
    InvalidVersionString(String),
    CannotMigrate {
        from: String,
        to: String,
    },
    CannotCreateArchive(String),
    CannotExtractArchive(String),
    CannotClone(String),
    CannotPush(String),
    CannotAddFile {
        file: String,  // rel_path
        message: String,
    },
    DirtyKnowledgeBase,
    DeprecatedConfig {
        key: String,
        message: String,
    },
    InvalidMergePrefix(String),
    MergeConflict(Uid),
    MPSCError(String),
    CannotDecodeUid,
    ModelNotSelected,
    InvalidModelName {
        name: String,
        candidates: Vec<String>,
    },

    /// The error message looks like
    /// "in order to do {action}, you have to enable feature {feature}."
    FeatureNotEnabled { action: String, feature: String },
    ApiKeyNotFound { env_var: Option<String> },

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

    #[cfg(feature = "csv")]
    /// see <https://docs.rs/csv/latest/csv/struct.Error.html>
    CsvError(csv::Error),

    /// see <https://docs.rs/url/latest/url/enum.ParseError.html>
    UrlParseError(url::ParseError),

    /// see <https://docs.rs/tokio/latest/tokio/task/struct.JoinError.html>
    JoinError(tokio::task::JoinError),

    #[cfg(feature = "pdf")]
    /// see <https://docs.rs/mupdf/latest/mupdf/error/enum.Error.html>
    MuPdfError(mupdf::Error),

    #[cfg(feature = "svg")]
    /// see <https://docs.rs/usvg/0.45.1/usvg/enum.Error.html>
    UsvgError(resvg::usvg::Error),

    #[cfg(feature = "svg")]
    /// see <https://docs.rs/png/latest/png/enum.EncodingError.html>
    PngEncodingError(png::EncodingError),

    /// <https://docs.rs/tera/latest/tera/struct.Error.html>
    TeraError(tera::Error),

    FileError(FileError),
    StdIoError(std::io::Error),
    FromUtf8Error,

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

#[cfg(feature = "csv")]
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
    fn from(_: FromUtf8Error) -> Error {
        Error::FromUtf8Error
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Error {
        Error::UrlParseError(e)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Error {
        Error::JoinError(e)
    }
}

#[cfg(feature = "pdf")]
impl From<mupdf::Error> for Error {
    fn from(e: mupdf::Error) -> Self {
        Error::MuPdfError(e)
    }
}

#[cfg(feature = "svg")]
impl From<resvg::usvg::Error> for Error {
    fn from(e: resvg::usvg::Error) -> Self {
        Error::UsvgError(e)
    }
}

// `png` crate is not for handling pngs. It's only because some functions in
// `resvg` returns `png::EncodingError`.
#[cfg(feature = "svg")]
impl From<png::EncodingError> for Error {
    fn from(e: png::EncodingError) -> Self {
        Error::PngEncodingError(e)
    }
}

impl From<tera::Error> for Error {
    fn from(e: tera::Error) -> Error {
        Error::TeraError(e)
    }
}

impl From<ApiError> for Error {
    fn from(e: ApiError) -> Self {
        match e {
            ApiError::JsonTypeError { expected, got } => Error::JsonTypeError { expected, got },
            ApiError::StdIoError(e) => Error::StdIoError(e),
            ApiError::ReqwestError(e) => Error::ReqwestError(e),
            ApiError::JsonSerdeError(e) => Error::JsonSerdeError(e),
            ApiError::TeraError(e) => Error::TeraError(e),
            ApiError::FileError(e) => Error::FileError(e),
            ApiError::ApiKeyNotFound { env_var } => Error::ApiKeyNotFound { env_var },
            ApiError::InvalidModelName { name, candidates } => Error::InvalidModelName { name, candidates },
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

impl From<ragit_pdl::SchemaParseError> for Error {
    fn from(e: ragit_pdl::SchemaParseError) -> Self {
        ragit_pdl::Error::from(e).into()
    }
}
