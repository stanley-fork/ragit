pub use super::{BuildConfig, Index};

// If a command dumps anything to stdout, its method must have `quiet: bool` argument.
mod add;
mod archive;
mod build;
mod check;
mod clone;
mod config;
mod gc;
mod ls;
mod merge;
mod meta;
mod migrate;
mod push;
mod recover;
mod remove;

pub use add::{AddMode, AddResult};
pub use merge::{MergeMode, MergeResult};
pub use migrate::{VersionInfo, get_compatibility_warning};
pub use recover::RecoverResult;
pub use remove::RemoveResult;
