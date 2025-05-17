pub use super::{BuildConfig, Index};
use std::io::Write;

// If a command dumps anything to stdout, its method must have `quiet: bool` argument.
mod add;
mod archive;
mod audit;
mod build;
mod check;
mod clone;
mod config;
mod gc;
mod ls;
mod merge;
mod meta;
mod migrate;
mod pull;
mod push;
mod recover;
mod remove;
mod uid;

pub use add::{AddMode, AddResult};
pub use audit::Audit;
pub use build::BuildResult;
pub use merge::{MergeMode, MergeResult};
pub use migrate::{VersionInfo, get_compatibility_warning};
pub use recover::RecoverResult;
pub use remove::RemoveResult;

pub fn erase_lines(n: usize) {
    if n != 0 {
        print!("\x1B[{n}A");
        print!("\x1B[J");
        std::io::stdout().flush().unwrap();
    }
}
