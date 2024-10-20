use serde::{Deserialize, Serialize};

pub const BUILD_CONFIG_FILE_NAME: &str = "build.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
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
    pub chunks_per_json: usize,

    // if the `.chunks` file is bigger than this (in bytes),
    // the file is compressed
    pub compression_threshold: u64,

    // 0 ~ 9
    pub compression_level: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            chunk_size: 4_000,
            slide_len: 1_000,
            image_size: 2_000,
            min_summary_len: 200,
            max_summary_len: 1000,
            chunks_per_json: 64,
            compression_threshold: 65536,
            compression_level: 3,
        }
    }
}
