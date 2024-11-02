pub use super::{BuildConfig, Index};

// functions in these modules are not supposed to call `Index::save_to_file`
mod add;
mod auto_recover;
mod build;
mod check;
mod config;
mod gc;
mod ls;
mod merge;
mod meta;
mod remove;
mod reset;

pub use add::{AddMode, AddResult};
