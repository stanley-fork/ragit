use crate::error::Error;
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use std::io::Read;

pub use super::erase_lines;

mod create;
mod extract;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum BlockType {
    Index,
    Chunk,
    ImageBytes,
    ImageDesc,
    Meta,
    Prompt,
    Config,
    Splitted,
    QueryHistory,
}

impl BlockType {
    pub fn to_byte(&self) -> u8 {
        match self {
            BlockType::Index => 0,
            BlockType::Chunk => 1,
            BlockType::ImageBytes => 2,
            BlockType::ImageDesc => 3,
            BlockType::Meta => 4,
            BlockType::Prompt => 5,
            BlockType::Config => 6,
            BlockType::Splitted => 7,
            BlockType::QueryHistory => 8,
        }
    }
}

impl TryFrom<u8> for BlockType {
    type Error = ();

    fn try_from(n: u8) -> Result<BlockType, ()> {
        match n {
            0 => Ok(BlockType::Index),
            1 => Ok(BlockType::Chunk),
            2 => Ok(BlockType::ImageBytes),
            3 => Ok(BlockType::ImageDesc),
            4 => Ok(BlockType::Meta),
            5 => Ok(BlockType::Prompt),
            6 => Ok(BlockType::Config),
            7 => Ok(BlockType::Splitted),
            8 => Ok(BlockType::QueryHistory),
            _ => Err(()),
        }
    }
}

fn compress(bytes: &[u8], level: u32) -> Result<Vec<u8>, Error> {
    let mut compressed = vec![];
    let mut gz = GzEncoder::new(bytes, Compression::new(level));
    gz.read_to_end(&mut compressed)?;
    Ok(compressed)
}

fn decompress(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let mut decompressed = vec![];
    let mut gz = GzDecoder::new(bytes);
    gz.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}
