use crate::{
    FileError,
    WriteMode,
    exists,
    file_size,
    rename,
    write_string,
};
use chrono::Local;
use std::sync::OnceLock;

static LOG_FILE_PATH: OnceLock<Option<String>> = OnceLock::new();
static DUMP_TO_STDOUT: OnceLock<bool> = OnceLock::new();
static DUMP_TO_STDERR: OnceLock<bool> = OnceLock::new();

pub fn initialize_log(
    dump_to_file: Option<String>,
    dump_to_stdout: bool,
    dump_to_stderr: bool,
    keep_previous_file: bool,
) -> Result<(), FileError> {
    if let Some(path) = &dump_to_file {
        if !keep_previous_file || !exists(path) {
            write_string(path, "", WriteMode::Atomic)?;
        }
    }

    LOG_FILE_PATH.set(dump_to_file).unwrap();
    DUMP_TO_STDOUT.set(dump_to_stdout).unwrap();
    DUMP_TO_STDERR.set(dump_to_stderr).unwrap();
    Ok(())
}

fn get_log_file_path() -> Option<String> {
    LOG_FILE_PATH.get().map(|p| p.clone()).unwrap_or(None)
}

pub fn write_log(owner: &str, msg: &str) {
    let dump_to_stdout = DUMP_TO_STDOUT.get().map(|b| *b).unwrap_or(false);
    let dump_to_stderr = DUMP_TO_STDERR.get().map(|b| *b).unwrap_or(false);
    let path = get_log_file_path();

    if path.is_none() && !dump_to_stdout && !dump_to_stderr {
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
        if let Ok(size) = file_size(&path) {
            if size > (1 << 20) {  // at most 1 MiB per log file
                let dst = format!("{path}-{}", Local::now().to_rfc3339().get(0..19).unwrap());

                if let Err(e) = rename(&path, &dst) {
                    eprintln!("error at `rename({path:?}, {dst:?})` at `write_log(...)`: {e:?}");
                }

                if let Err(e) = write_string(&path, "", WriteMode::Atomic) {
                    eprintln!("error at `write_string({path:?}, \"\", WriteMode::Atomic)` at `write_log(...)`: {e:?}");
                }
            }
        }

        if let Err(e) = write_string(
            &path,
            &message,
            WriteMode::AlwaysAppend,
        ) {
            eprintln!("error at `write_string({path:?}, ..)` at `write_log(...)`: {e:?}");
        }
    }

    if dump_to_stdout {
        print!("{message}");
    }

    if dump_to_stderr {
        eprint!("{message}");
    }
}
