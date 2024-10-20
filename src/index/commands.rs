pub use super::{Config, Index};

// functions in these modules are not supposed to call `Index::save_to_file`
mod add;
mod check;
mod gc;
mod get;
mod ls;
mod merge;
mod meta;
mod remove;
mod reset;
mod set;

pub use add::{AddMode, AddResult};
