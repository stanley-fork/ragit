use super::Config;
use crate::api_config::ApiConfig;
use crate::chunk::{BuildInfo, Chunk};
use crate::error::Error;
use ragit_api::{ImageType, MessageContent};
use ragit_fs::{
    extension,
    read_bytes,
};
use sha3::{Digest, Sha3_256};
use std::collections::{HashMap, VecDeque};

mod image;
mod line_reader;
mod markdown;
mod plain_text;

pub use image::normalize_image;
pub use line_reader::LineReader;
pub use markdown::MarkdownReader;
pub use plain_text::PlainTextReader;

pub type Path = String;

pub trait FileReaderImpl {
    fn new(path: &str, config: &Config) -> Result<Self, Error> where Self: Sized;
    fn pop_all_tokens(&mut self) -> Result<Vec<AtomicToken>, Error>;

    /// It reads `path` and load more tokens to memory. If the file is small enough,
    /// it may load the entire file at once.
    fn load_tokens(&mut self) -> Result<(), Error>;

    /// It has nothing to do with `pop_all_tokens`. It tells whether `load_tokens` can load
    /// more tokens or not.
    fn has_more_to_read(&self) -> bool;

    /// Every chunk starts with `chunk_header` and ends with `chunk_footer`.
    /// They're empty by default. Headers and footers are not counted when calculating size of chunks.
    /// So, make sure to keep headers and footers small enough.
    fn chunk_header(&self) -> Vec<AtomicToken> { vec![] }
    fn chunk_footer(&self) -> Vec<AtomicToken> { vec![] }

    /// It's used by `BuildInfo`. It's used to distinguish `FileReader`s.
    fn key(&self) -> String;
}

pub struct FileReader {  // of a single file
    rel_path: Path,
    inner: Box<dyn FileReaderImpl>,
    buffer: VecDeque<AtomicToken>,
    curr_buffer_size: usize,
    pub images: HashMap<String, Vec<u8>>,
    config: Config,

    // index IN a file, not OF a file
    file_index: usize,
}

impl FileReader {
    pub fn new(rel_path: Path, real_path: Path, config: Config) -> Result<Self, Error> {
        let inner = match extension(&rel_path)?.unwrap_or(String::new()).to_ascii_lowercase().as_str() {
            "md" => Box::new(MarkdownReader::new(&real_path, &config)?) as Box<dyn FileReaderImpl>,
            
            // a newline character isn't always a row-separator in csv, but that's okay
            // because LLM reads the file, not *parse* the file.
            "csv" => Box::new(LineReader::new(&real_path, &config)?.set_header_length(1)),
            "jsonl" => Box::new(LineReader::new(&real_path, &config)?.set_header_length(0)),

            // "pdf" => Box::new(PdfReader::new(&real_path, &config)?),
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
            file_index: 0,
        })
    }

    pub fn can_generate_chunk(&self) -> bool {
        !self.buffer.is_empty() || self.inner.has_more_to_read()
    }

    pub async fn generate_chunk(
        &mut self,
        api_config: &ApiConfig,
        pdl: &str,
        build_info: BuildInfo,
    ) -> Result<Chunk, Error> {
        self.fill_buffer_until_chunks(2)?;

        // prevent creating too small chunk
        let next_chunk_size = if self.config.chunk_size < self.curr_buffer_size && self.curr_buffer_size < self.config.chunk_size * 2 {
            self.curr_buffer_size / 2
        } else {
            self.config.chunk_size
        };

        let mut chunk_deque = VecDeque::new();
        let mut curr_chunk_size = 0;

        while curr_chunk_size < next_chunk_size && !self.buffer.is_empty() {
            let token = self.buffer.pop_front().unwrap();
            self.curr_buffer_size -= token.len(self.config.image_size);
            curr_chunk_size += token.len(self.config.image_size);
            chunk_deque.push_back(token);
        }

        // if there's no remaining token, there's no need for sliding window
        if !self.buffer.is_empty() {
            let mut sliding_window_deque = VecDeque::new();
            let mut curr_sliding_window_size = 0;

            while curr_sliding_window_size < self.config.slide_len && !chunk_deque.is_empty() {
                let token = chunk_deque.pop_back().unwrap();
                curr_sliding_window_size += token.len(self.config.image_size);
                self.buffer.push_front(token.clone());
                self.curr_buffer_size += token.len(self.config.image_size);
                sliding_window_deque.push_front(token);
            }

            for token in sliding_window_deque.into_iter() {
                chunk_deque.push_back(token);
            }
        }

        // in order to prevent headers and footers from being
        // included to the sliding window, they're pushed later
        for token in self.inner.chunk_header().into_iter().rev() {
            chunk_deque.push_front(token);
        }

        for token in self.inner.chunk_footer() {
            chunk_deque.push_back(token);
        }

        let tokens = merge_tokens(chunk_deque);

        self.file_index += 1;
        let chunk = Chunk::create_chunk_from(
            &tokens,
            &self.config,
            self.rel_path.clone(),
            self.file_index,
            api_config,
            pdl,
            build_info,
        ).await;

        for token in tokens.into_iter() {
            if let AtomicToken::Image(Image { key, bytes, image_type }) = token {
                let bytes = normalize_image(bytes, image_type)?;
                self.images.insert(key, bytes);
            }
        }

        if let Some(ms) = api_config.sleep_after_llm_call {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        chunk
    }

    fn fill_buffer_until_chunks(&mut self, chunk_count: usize) -> Result<(), Error> {
        loop {
            if self.curr_buffer_size >= chunk_count * self.config.chunk_size {
                return Ok(());
            }

            self.try_fill_buffer()?;

            if !self.inner.has_more_to_read() {
                return Ok(());
            }
        }
    }

    fn try_fill_buffer(&mut self) -> Result<bool, Error> {  // returns whether something has been pushed to the buffer
        self.inner.load_tokens()?;
        let mut has_tokens = false;

        for token in self.inner.pop_all_tokens()? {
            has_tokens = true;
            self.curr_buffer_size += token.len(self.config.image_size);
            self.buffer.push_back(token);
        }

        Ok(has_tokens)
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
                }

                result.push(token);
            },
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

#[derive(Clone, Debug)]
pub enum AtomicToken {
    String {
        data: String,
        char_len: usize,
    },
    Image(Image),
}

impl AtomicToken {
    pub fn len(&self, image_size: usize) -> usize {
        match self {
            AtomicToken::String { char_len, .. } => *char_len,
            AtomicToken::Image(_) => image_size,
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
        }
    }
}

// TODO: it's not good idea to derive `Debug` on that big bytes
#[derive(Clone, Debug)]
pub struct Image {
    pub key: String,  // unique ID
    pub image_type: ImageType,
    pub bytes: Vec<u8>,
}

pub fn get_file_hash(path: &Path) -> Result<String, Error> {
    // TODO: don't read the entire file at once
    let file_content = read_bytes(path)?;
    let mut hasher = Sha3_256::new();
    hasher.update(&file_content);

    Ok(format!("{:08}_{:064x}", file_content.len(), hasher.finalize()))
}
