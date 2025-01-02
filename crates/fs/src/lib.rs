#![allow(dead_code)]

mod log;

pub use crate::log::{initialize_log_file, set_log_file_path, write_log};

use std::collections::hash_map;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// ```nohighlight
///       File Already Exists    File Does not Exist
///
///     AA       Append                  Dies
///    AoC       Append                 Create
///    CoT      Truncate                Create
///     AC        Dies                  Create
/// ```
pub enum WriteMode {
    AlwaysAppend,
    AppendOrCreate,
    CreateOrTruncate,
    AlwaysCreate,
}

impl From<WriteMode> for OpenOptions {
    fn from(m: WriteMode) -> OpenOptions {
        let mut result = OpenOptions::new();

        match m {
            WriteMode::AlwaysAppend => { result.append(true); },
            WriteMode::AppendOrCreate => { result.append(true).create(true); },
            WriteMode::CreateOrTruncate => { result.write(true).truncate(true).create(true); },
            WriteMode::AlwaysCreate => { result.write(true).create_new(true); },
        }

        result
    }
}

/// It never reads more than `to - from` bytes.
/// If it fails to read from `from`, that's an error.
/// If it fails to read to `to`, that's not an error.
pub fn read_bytes_offset(path: &str, from: u64, to: u64) -> Result<Vec<u8>, FileError> {
    assert!(to >= from);

    match File::open(path) {
        Err(e) => Err(FileError::from_std(e, path)),
        Ok(mut f) => match f.seek(SeekFrom::Start(from)) {
            Err(e) => Err(FileError::from_std(e, path)),
            Ok(_) => {
                let mut handle = f.take(to - from);
                let mut buffer = Vec::with_capacity((to - from) as usize);

                if let Err(e) = handle.read_to_end(&mut buffer) {
                    return Err(FileError::from_std(e, path));
                }

                Ok(buffer)
            },
        },
    }
}

pub fn read_bytes(path: &str) -> Result<Vec<u8>, FileError> {
    fs::read(path).map_err(|e| FileError::from_std(e, path))
}

pub fn read_string(path: &str) -> Result<String, FileError> {
    let mut s = String::new();

    match File::open(path) {
        Err(e) => Err(FileError::from_std(e, path)),
        Ok(mut f) => match f.read_to_string(&mut s) {
            Ok(_) => Ok(s),
            Err(e) => Err(FileError::from_std(e, path)),
        }
    }
}

pub fn write_bytes(path: &str, bytes: &[u8], write_mode: WriteMode) -> Result<(), FileError> {
    let option: OpenOptions = write_mode.into();

    match option.open(path) {
        Ok(mut f) => match f.write_all(bytes) {
            Ok(_) => Ok(()),
            Err(e) => Err(FileError::from_std(e, path)),
        },
        Err(e) => Err(FileError::from_std(e, path)),
    }
}

pub fn write_string(path: &str, s: &str, write_mode: WriteMode) -> Result<(), FileError> {
    write_bytes(path, s.as_bytes(), write_mode)
}

/// `a/b/c.d` -> `c`
pub fn file_name(path: &str) -> Result<String, FileError> {
    let path_buf = PathBuf::from_str(path).unwrap();  // it's infallible

    match path_buf.file_stem() {
        None => Ok(String::new()),
        Some(s) => match s.to_str() {
            Some(ext) => Ok(ext.to_string()),
            None => Err(FileError::os_str_err(s.to_os_string())),
        }
    }
}

/// `a/b/c.d` -> `d`
pub fn extension(path: &str) -> Result<Option<String>, FileError> {
    let path_buf = PathBuf::from_str(path).unwrap();  // it's infallible

    match path_buf.extension() {
        None => Ok(None),
        Some(s) => match s.to_str() {
            Some(ext) => Ok(Some(ext.to_string())),
            None => Err(FileError::os_str_err(s.to_os_string())),
        }
    }
}

/// `a/b/c.d` -> `c.d`
pub fn basename(path: &str) -> Result<String, FileError> {
    let path_buf = PathBuf::from_str(path).unwrap();  // it's infallible

    match path_buf.file_name() {
        None => Ok(String::new()),  // when the path terminates in `..`
        Some(s) => match s.to_str() {
            Some(ext) => Ok(ext.to_string()),
            None => Err(FileError::os_str_err(s.to_os_string())),
        }
    }
}

/// `a/b/`, `c.d` -> `a/b/c.d`
pub fn join(path: &str, child: &str) -> Result<String, FileError> {
    let mut path_buf = PathBuf::from_str(path).unwrap();  // Infallible
    let child = PathBuf::from_str(child).unwrap();  // Infallible

    path_buf.push(child);

    match path_buf.to_str() {
        Some(result) => Ok(result.to_string()),
        None => Err(FileError::os_str_err(path_buf.into_os_string())),
    }
}

pub fn join3(path1: &str, path2: &str, path3: &str) -> Result<String, FileError> {
    join(
        path1,
        &join(path2, path3)?,
    )
}

pub fn join4(path1: &str, path2: &str, path3: &str, path4: &str) -> Result<String, FileError> {
    join(
        &join(path1, path2)?,
        &join(path3, path4)?,
    )
}

/// `a/b/c.d, e` -> `a/b/c.e`
pub fn set_extension(path: &str, ext: &str) -> Result<String, FileError> {
    let mut path_buf = PathBuf::from_str(path).unwrap();  // Infallible

    if path_buf.set_extension(ext) {
        match path_buf.to_str() {
            Some(result) => Ok(result.to_string()),
            None => Err(FileError::os_str_err(path_buf.into_os_string())),
        }
    } else {
        // has no filename
        Ok(path.to_string())
    }
}

/// It returns `false` if `path` doesn't exist
pub fn is_dir(path: &str) -> bool {
    PathBuf::from_str(path).map(|path| path.is_dir()).unwrap_or(false)
}

pub fn exists(path: &str) -> bool {
    PathBuf::from_str(path).map(|path| path.exists()).unwrap_or(false)
}

/// `a/b/c.d` -> `a/b/`
pub fn parent(path: &str) -> Result<String, FileError> {
    let std_path = Path::new(path);

    std_path.parent().map(
        |p| p.to_string_lossy().to_string()
    ).ok_or_else(
        || FileError::unknown(
            String::from("function `parent` died"),
            Some(path.to_string()),
        )
    )
}

pub fn create_dir(path: &str) -> Result<(), FileError> {
    fs::create_dir(path).map_err(|e| FileError::from_std(e, path))
}

pub fn create_dir_all(path: &str) -> Result<(), FileError> {
    fs::create_dir_all(path).map_err(|e| FileError::from_std(e, path))
}

pub fn rename(from: &str, to: &str) -> Result<(), FileError> {
    fs::rename(from, to).map_err(|e| FileError::from_std(e, from))
}

pub fn copy_dir(src: &str, dst: &str) -> Result<(), FileError> {
    create_dir_all(dst)?;

    // TODO: how about links?
    for e in read_dir(src, false)? {
        let new_dst = join(dst, &basename(&e)?)?;

        if is_dir(&e) {
            create_dir_all(&new_dst)?;
            copy_dir(&e, &new_dst)?;
        }

        else {
            copy_file(&e, &new_dst)?;
        }
    }

    Ok(())
}

/// It returns the total number of bytes copied.
pub fn copy_file(src: &str, dst: &str) -> Result<u64, FileError> {
    std::fs::copy(src, dst).map_err(|e| FileError::from_std(e, src))  // TODO: how about dst?
}

// it only returns the hash value of the modified time
pub fn last_modified(path: &str) -> Result<u64, FileError> {
    match fs::metadata(path) {
        Ok(m) => match m.modified() {
            Ok(m) => {
                let mut hasher = hash_map::DefaultHasher::new();
                m.hash(&mut hasher);
                let hash = hasher.finish();

                Ok(hash)
            },
            Err(e) => Err(FileError::from_std(e, path)),
        },
        Err(e) => Err(FileError::from_std(e, path)),
    }
}

pub fn file_size(path: &str) -> Result<u64, FileError> {
    match fs::metadata(path) {
        Ok(m) => Ok(m.len()),
        Err(e) => Err(FileError::from_std(e, path)),
    }
}

pub fn read_dir(path: &str, sort: bool) -> Result<Vec<String>, FileError> {
    match fs::read_dir(path) {
        Err(e) => Err(FileError::from_std(e, path)),
        Ok(entries) => {
            let mut result = vec![];

            for entry in entries {
                match entry {
                    Err(e) => {
                        return Err(FileError::from_std(e, path));
                    },
                    Ok(e) => {
                        if let Some(ee) = e.path().to_str() {
                            result.push(ee.to_string());
                        }
                    },
                }
            }

            if sort {
                result.sort();
            }

            Ok(result)
        }
    }
}

pub fn remove_file(path: &str) -> Result<(), FileError> {
    fs::remove_file(path).map_err(|e| FileError::from_std(e, path))
}

pub fn remove_dir(path: &str) -> Result<(), FileError> {
    fs::remove_dir(path).map_err(|e| FileError::from_std(e, path))
}

pub fn remove_dir_all(path: &str) -> Result<(), FileError> {
    fs::remove_dir_all(path).map_err(|e| FileError::from_std(e, path))
}

pub fn into_abs_path(path: &str) -> Result<String, FileError> {
    let std_path = Path::new(path);

    if std_path.is_absolute() {
        Ok(path.to_string())
    }

    else {
        Ok(join(
            &current_dir()?,
            path,
        )?)
    }
}

pub fn current_dir() -> Result<String, FileError> {
    let cwd = std::env::current_dir().map_err(|e| FileError::from_std(e, "."))?;

    match cwd.to_str() {
        Some(cwd) => Ok(cwd.to_string()),
        None => Err(FileError::os_str_err(cwd.into_os_string())),
    }
}

pub fn diff(path: &str, base: &str) -> Result<String, FileError> {
    match pathdiff::diff_paths(path, base) {
        Some(path) => match path.to_str() {
            Some(path) => Ok(path.to_string()),
            None => Err(FileError::os_str_err(path.into_os_string())),
        },
        None => Err(FileError::cannot_diff_path(path.to_string(), base.to_string())),
    }
}

pub fn normalize(path: &str) -> Result<String, FileError> {
    let mut result = vec![];
    let path = path.replace("\\", "/");

    for component in path.split("/") {
        match component {
            c if c == "." => {},
            c if c == ".." => if result.is_empty() {
                result.push(c.to_string());
            } else {
                result.pop();
            },
            c => { result.push(c.to_string()); },
        }
    }

    Ok(result.join("/"))
}

#[derive(Clone,  PartialEq)]
pub struct FileError {
    pub kind: FileErrorKind,
    pub given_path: Option<String>,
}

impl FileError {
    pub fn from_std(e: io::Error, given_path: &str) -> Self {
        let kind = match e.kind() {
            io::ErrorKind::NotFound => FileErrorKind::FileNotFound,
            io::ErrorKind::PermissionDenied => FileErrorKind::PermissionDenied,
            io::ErrorKind::AlreadyExists => FileErrorKind::AlreadyExists,
            e => FileErrorKind::Unknown(format!("unknown error: {e:?}")),
        };

        FileError {
            kind,
            given_path: Some(given_path.to_string()),
        }
    }

    pub(crate) fn os_str_err(os_str: OsString) -> Self {
        FileError {
            kind: FileErrorKind::OsStrErr(os_str),
            given_path: None,
        }
    }

    pub(crate) fn cannot_diff_path(path: String, base: String) -> Self {
        FileError {
            kind: FileErrorKind::CannotDiffPath(path.to_string(), base),
            given_path: Some(path),
        }
    }

    pub fn unknown(msg: String, path: Option<String>) -> Self {
        FileError {
            kind: FileErrorKind::Unknown(msg),
            given_path: path,
        }
    }

    pub fn render_error(&self) -> String {
        let path = self.given_path.as_ref().map(|p| p.to_string()).unwrap_or(String::new());

        match &self.kind {
            FileErrorKind::FileNotFound => format!(
                "file not found: `{path}`"
            ),
            FileErrorKind::PermissionDenied => format!(
                "permission denied: `{path}`"
            ),
            FileErrorKind::AlreadyExists => format!(
                "file already exists: `{path}`"
            ),
            FileErrorKind::CannotDiffPath(path, base) => format!(
                "cannot calc diff: `{path}` and `{base}`"
            ),
            FileErrorKind::Unknown(msg) => format!(
                "unknown file error: `{msg}`"
            ),
            FileErrorKind::OsStrErr(os_str) => format!(
                "error converting os_str: `{os_str:?}`"
            ),
        }
    }
}

impl fmt::Debug for FileError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.render_error())
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.render_error())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileErrorKind {
    FileNotFound,
    PermissionDenied,
    AlreadyExists,
    CannotDiffPath(String, String),
    Unknown(String),
    OsStrErr(OsString),
}
