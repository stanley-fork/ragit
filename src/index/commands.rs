pub use super::{BuildConfig, Index};

// functions in these modules are not supposed to call `Index::save_to_file`
mod add;
mod auto_recover;
mod build;
mod check;
mod clone;
mod config;
mod ext;
mod gc;
mod ls;
mod meta;
mod migrate;
mod remove;
mod reset;

pub use add::{AddMode, AddResult};
pub use auto_recover::AutoRecoverResult;
pub use meta::METADATA_FILE_NAME;
