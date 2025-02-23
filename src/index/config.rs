use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct BuildConfig {
    // it's not a max_chunk_size, and it's impossible to make every chunk have the same size because
    // 1. an image cannot be splitted
    // 2. different files cannot be merged
    // but it's guaranteed that a chunk is never bigger than chunk_size * 2
    pub chunk_size: usize,

    pub slide_len: usize,

    // an image is treated like an N characters string
    // this is N
    pub image_size: usize,

    pub min_summary_len: usize,
    pub max_summary_len: usize,

    // If it's set, `rag build` panics if there's any error with a file.
    // For example, if there's an invalid utf-8 character `PlainTextReader` would die.
    // If it cannot follow a link of an image in a markdown file, it would die.
    // You don't need this option unless you're debugging ragit itself.
    pub strict_file_reader: bool,

    // if the `.chunks` file is bigger than this (in bytes),
    // the file is compressed
    pub compression_threshold: u64,

    // 0 ~ 9
    pub compression_level: u32,
}

impl Default for BuildConfig {
    fn default() -> Self {
        BuildConfig {
            chunk_size: 4_000,
            slide_len: 1_000,
            image_size: 2_000,
            min_summary_len: 200,
            max_summary_len: 1000,
            strict_file_reader: false,
            compression_threshold: 2048,
            compression_level: 3,
        }
    }
}
