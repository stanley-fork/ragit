use super::BuildConfig;
use crate::chunk::{Chunk, ChunkBuildInfo, ChunkSchema};
use crate::error::Error;
use crate::index::Index;
use crate::uid::Uid;
use ragit_fs::extension;
use ragit_pdl::MessageContent;
use std::collections::{HashMap, VecDeque};

mod csv;
mod image;
mod line;
mod markdown;
mod plain_text;
mod pdf;

pub use csv::CsvReader;
pub use image::{Image, ImageDescription, ImageReader, normalize_image};
pub use line::LineReader;
pub use markdown::MarkdownReader;
pub use plain_text::PlainTextReader;
pub use pdf::PdfReader;

pub type Path = String;

pub trait FileReaderImpl {
    fn new(path: &str, config: &BuildConfig) -> Result<Self, Error> where Self: Sized;
    fn pop_all_tokens(&mut self) -> Result<Vec<AtomicToken>, Error>;

    /// It reads `path` and load more tokens to memory. If the file is small enough,
    /// it may load the entire file at once.
    fn load_tokens(&mut self) -> Result<(), Error>;

    /// It has nothing to do with `pop_all_tokens`. It tells whether `load_tokens` can load
    /// more tokens or not.
    fn has_more_to_read(&self) -> bool;

    /// It's used by `BuildInfo`. It's used to distinguish `FileReader`s.
    fn key(&self) -> String;
}

pub struct FileReader {  // of a single file
    rel_path: Path,
    inner: Box<dyn FileReaderImpl + Send>,
    buffer: VecDeque<AtomicToken>,
    curr_buffer_size: usize,
    pub images: HashMap<Uid, Vec<u8>>,
    config: BuildConfig,
}

impl FileReader {
    pub fn new(rel_path: Path, real_path: Path, config: BuildConfig) -> Result<Self, Error> {
        // TODO: use a config file, instead of hard-coding the extensions
        let inner = match extension(&rel_path)?.unwrap_or(String::new()).to_ascii_lowercase().as_str() {
            "md" => Box::new(MarkdownReader::new(&real_path, &config)?) as Box<dyn FileReaderImpl + Send>,
            "png" | "jpg" | "jpeg" | "gif" | "webp" => Box::new(ImageReader::new(&real_path, &config)?),
            "jsonl" => Box::new(LineReader::new(&real_path, &config)?),
            "csv" => Box::new(CsvReader::new(&real_path, &config)?),
            "pdf" => Box::new(PdfReader::new(&real_path, &config)?),

            // "py" | "rs" => Box::new(CodeReader::new(&real_path, &config)?),

            // all the unknown extensions are treated as plain texts
            _ => Box::new(PlainTextReader::new(&real_path, &config)?),
        };

        Ok(FileReader {
            rel_path,
            inner,
            buffer: VecDeque::new(),
            curr_buffer_size: 0,
            images: HashMap::new(),
            config,
        })
    }

    pub fn can_generate_chunk(&self) -> bool {
        !self.buffer.is_empty() || self.inner.has_more_to_read()
    }

    /// It moves the cursor and generates `Vec<AtomicToken>` for the next chunk.
    /// It also collects images in the next chunk.
    pub fn next_chunk(&mut self) -> Result<Vec<AtomicToken>, Error> {
        self.fill_buffer_until_chunks(2)?;

        // prevent creating too small chunk
        let next_chunk_size = if self.config.chunk_size < self.curr_buffer_size && self.curr_buffer_size < self.config.chunk_size * 2 {
            self.curr_buffer_size / 2
        } else {
            self.config.chunk_size
        };

        let mut chunk_deque = VecDeque::new();
        let mut curr_chunk_size = 0;
        let mut has_separator = false;

        // step 1. collect tokens for a chunk
        while curr_chunk_size < next_chunk_size && !self.buffer.is_empty() {
            let token = self.buffer.pop_front().unwrap();

            if let AtomicToken::Separator = &token {
                has_separator = true;
                break;
            }

            self.curr_buffer_size -= token.len(self.config.image_size);
            curr_chunk_size += token.len(self.config.image_size);
            chunk_deque.push_back(token);
        }

        // step 2. create a sliding window
        // if there's no remaining token, there's no need for sliding window
        // if the chunk consists of a single token, there's no point in making a sliding window
        if !has_separator && (!self.buffer.is_empty() || chunk_deque.len() == 1) {
            let mut sliding_window_deque = VecDeque::new();
            let mut curr_sliding_window_size = 0;

            while curr_sliding_window_size < self.config.slide_len && !chunk_deque.is_empty() {
                let token = chunk_deque.pop_back().unwrap();
                curr_sliding_window_size += token.len(self.config.image_size);
                self.buffer.push_front(token.clone());
                self.curr_buffer_size += token.len(self.config.image_size);
                sliding_window_deque.push_front(token);
            }

            // prevent infinite loop
            if curr_sliding_window_size == curr_chunk_size {
                let token = self.buffer.pop_front().unwrap();
                self.curr_buffer_size -= token.len(self.config.image_size);
            }

            for token in sliding_window_deque.into_iter() {
                chunk_deque.push_back(token);
            }
        }

        let tokens = merge_tokens(chunk_deque);

        for token in tokens.iter() {
            if let AtomicToken::Image(Image { uid, bytes, .. }) = token {
                self.images.insert(*uid, bytes.clone());
            }
        }

        Ok(tokens)
    }

    pub async fn generate_chunk(
        &mut self,
        index: &Index,
        build_info: ChunkBuildInfo,
        previous_turn: Option<(Chunk, ChunkSchema)>,
        index_in_file: usize,
    ) -> Result<Chunk, Error> {
        let tokens = self.next_chunk()?;
        let chunk = Chunk::create_chunk_from(
            index,
            &tokens,
            self.rel_path.clone(),
            index_in_file,
            build_info,
            previous_turn,
        ).await;

        if let Some(ms) = index.api_config.sleep_after_llm_call {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        chunk
    }

    fn fill_buffer_until_chunks(&mut self, chunk_count: usize) -> Result<(), Error> {
        loop {
            if self.curr_buffer_size >= chunk_count * self.config.chunk_size {
                return Ok(());
            }

            self.inner.load_tokens()?;

            for token in self.inner.pop_all_tokens()? {
                self.curr_buffer_size += token.len(self.config.image_size);
                self.buffer.push_back(token);
            }

            if !self.inner.has_more_to_read() {
                return Ok(());
            }
        }
    }

    pub fn file_reader_key(&self) -> String {
        self.inner.key()
    }
}

fn merge_tokens(tokens: VecDeque<AtomicToken>) -> Vec<AtomicToken> {
    let mut buffer = vec![];
    let mut result = vec![];

    for token in tokens.into_iter() {
        match token {
            AtomicToken::String { data, .. } => { buffer.push(data); },
            AtomicToken::Image(_) => {
                if !buffer.is_empty() {
                    let s = buffer.concat();
                    result.push(AtomicToken::String {
                        char_len: s.chars().count(),
                        data: s,
                    });
                    buffer = vec![];
                }

                result.push(token);
            },

            // not rendered
            AtomicToken::Separator => {},
        }
    }

    if !buffer.is_empty() {
        let s = buffer.concat();
        result.push(AtomicToken::String {
            char_len: s.chars().count(),
            data: s,
        });
    }

    result
}

#[derive(Clone, Debug, PartialEq)]
pub enum AtomicToken {
    String {
        data: String,
        char_len: usize,
    },
    Image(Image),

    // It's an invisible AtomicToken.
    // An AtomicToken before a separator and
    // after a separator will never belong to the
    // same chunk.
    Separator,
}

impl AtomicToken {
    pub fn len(&self, image_size: usize) -> usize {
        match self {
            AtomicToken::String { char_len, .. } => *char_len,
            AtomicToken::Image(_) => image_size,
            AtomicToken::Separator => 0,
        }
    }
}

impl From<AtomicToken> for MessageContent {
    fn from(d: AtomicToken) -> Self {
        match d {
            AtomicToken::String { data, .. } => MessageContent::String(data),
            AtomicToken::Image(Image { image_type, bytes, ..}) => MessageContent::Image {
                image_type,
                bytes,
            },

            // this branch is not supposed to be reached
            AtomicToken::Separator => MessageContent::String(String::new()),
        }
    }
}
