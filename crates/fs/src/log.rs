use crate::{
    exists,
    write_string,
    FileError,
    WriteMode,
};
use chrono::offset::Local;

static mut LOG_FILE_PATH: Option<[u8; 1024]> = None;
static mut LOG_FILE_PATH_LEN: usize = 0;

pub fn set_log_file_path(path: Option<String>) {
    unsafe {
        if let Some(path) = path {
            let mut bytes = [0; 1024];

            for (i, c) in path.as_bytes().iter().enumerate() {
                bytes[i] = *c;
            }

            LOG_FILE_PATH_LEN = path.len();
            LOG_FILE_PATH = Some(bytes);
        }

        else {
            LOG_FILE_PATH = None;
        }
    }
}

fn get_log_file_path() -> Option<String> {
    unsafe {
        LOG_FILE_PATH.map(|bytes| String::from_utf8_lossy(&bytes[..LOG_FILE_PATH_LEN]).to_string())
    }
}

pub fn initialize_log_file(path: &str, remove_existing_file: bool) -> Result<(), FileError> {
    if remove_existing_file {
        if exists(path) {
            // TODO: append to the old file, instead of overwriting it
            if let Err(e) = std::fs::copy(
                path,
                &format!("{path}-backup"),
            ) {
                return Err(FileError::from_std(e, path));  // TODO: which path?
            }
        }

        write_string(path, "", WriteMode::CreateOrTruncate)?;
    }

    Ok(())
}

pub fn write_log(owner: &str, msg: &str) {
    if let Some(path) = get_log_file_path() {
        write_string(
            &path,
            &format!(
                "{} | {} | {msg}\n",
                Local::now().to_rfc2822(),
                if owner.len() < 32 {
                    format!("{}{owner}", " ".repeat(32 - owner.len()))
                } else {
                    owner.to_string()
                },
            ),
            WriteMode::AlwaysAppend,
        ).unwrap();
    }
}
