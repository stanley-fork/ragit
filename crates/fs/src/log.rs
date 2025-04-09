use crate::{
    FileError,
    WriteMode,
    copy_file,
    exists,
    write_string,
};
use chrono::Local;

const BUFFER_LEN: usize = 2048;
static mut LOG_FILE_PATH: Option<[u8; BUFFER_LEN]> = None;
static mut LOG_FILE_PATH_LEN: usize = 0;
static mut DUMP_TO_STDOUT: bool = false;
static mut DUMP_TO_STDERR: bool = false;

pub fn initialize_log(
    dump_to_file: Option<String>,
    dump_to_stdout: bool,
    dump_to_stderr: bool,
    keep_previous_file: bool,
) -> Result<(), FileError> {
    unsafe {
        if let Some(path) = dump_to_file {
            if path.len() > BUFFER_LEN {
                panic!("log path is too long: `{path}`");
            }

            if !keep_previous_file {
                if exists(&path) {
                    copy_file(&path, &format!("{path}-backup"))?;
                }

                write_string(&path, "", WriteMode::Atomic)?;
            }

            let mut bytes = [0; BUFFER_LEN];

            for (i, c) in path.as_bytes().iter().enumerate() {
                bytes[i] = *c;
            }

            LOG_FILE_PATH_LEN = path.len();
            LOG_FILE_PATH = Some(bytes);
        }

        DUMP_TO_STDOUT = dump_to_stdout;
        DUMP_TO_STDERR = dump_to_stderr;
    }

    Ok(())
}

fn get_log_file_path() -> Option<String> {
    unsafe {
        LOG_FILE_PATH.map(|bytes| String::from_utf8_lossy(&bytes[..LOG_FILE_PATH_LEN]).to_string())
    }
}

pub fn write_log(owner: &str, msg: &str) {
    unsafe {
        let path = get_log_file_path();

        if path.is_none() && !DUMP_TO_STDOUT && !DUMP_TO_STDERR {
            return;
        }

        let message = format!(
            "{} | {} | {msg}\n",
            Local::now().to_rfc2822(),
            if owner.len() < 32 {
                format!("{}{owner}", " ".repeat(32 - owner.len()))
            } else {
                owner.to_string()
            },
        );

        if let Some(path) = path {
            write_string(
                &path,
                &message,
                WriteMode::AlwaysAppend,
            ).unwrap();
        }

        if DUMP_TO_STDOUT {
            print!("{message}");
        }

        if DUMP_TO_STDERR {
            eprint!("{message}");
        }
    }
}
