use std::sync::OnceLock;

pub mod cli;
pub mod config;
pub mod error;
pub mod macros;
pub mod methods;
pub mod models;
pub mod utils;

pub static CONFIG: OnceLock<crate::config::Config> = OnceLock::new();
