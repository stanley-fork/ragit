use super::BuildConfig;
use crate::chunk::{Chunk, ChunkBuildInfo, ChunkExtraInfo, ChunkSchema};
use crate::error::Error;
use crate::index::Index;
use crate::uid::Uid;
use ragit_fs::extension;
use ragit_pdl::{MessageContent, ImageType};
use std::collections::{HashMap, VecDeque};

mod csv;
mod image;
mod line;
mod markdown;
mod plain_text;
mod pdf;

pub use csv::CsvReader;
pub use image::{Image, ImageDescription, ImageReader};
pub use line::LineReader;
pub use markdown::MarkdownReader;
pub use plain_text::PlainTextReader;
pub use pdf::PdfReader;

pub type Path = String;

/// Generic file reader.
/// A file reader reads a file and creates a sequence of `AtomicToken`.
///
/// Ragit will call `load_tokens` and `pop_all_tokens` until `has_more_to_read`
/// is false. It's designed like this because some files are too big to load to
/// memory at once.
pub trait FileReaderImpl {
    fn new(path: &str, config: &BuildConfig) -> Result<Self, Error> where Self: Sized;

    /// `load_tokens` loads tokens to buffer. This method *empties* and returns the buffer.
    /// You don't have to care about the length of its returned vector. If it contains
    /// too many tokens, ragit will split them into multiple chunks. If it returns a
    /// too small vector, ragit will call `load_tokens` again, unless `self.has_more_to_read`
    /// is false.
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

    // this is a cache, purely for optimizing `fetch_images_from_web()`
    fetched_images: HashMap<String, Uid>,  // HashMap<url, image_uid>
}

impl FileReader {
    pub fn new(rel_path: Path, real_path: Path, config: BuildConfig) -> Result<Self, Error> {
        // TODO: use a config file, instead of hard-coding the extensions
        let inner = match extension(&rel_path)?.unwrap_or(String::new()).to_ascii_lowercase().as_str() {
            "md" => Box::new(MarkdownReader::new(&real_path, &config)?) as Box<dyn FileReaderImpl + Send>,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => Box::new(ImageReader::new(&real_path, &config)?),
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
            fetched_images: HashMap::new(),
        })
    }

    pub fn can_generate_chunk(&self) -> bool {
        !self.buffer.is_empty() || self.inner.has_more_to_read()
    }

    /// It moves the cursor and generates `Vec<AtomicToken>` for the next chunk.
    /// It also collects images in the next chunk.
    pub fn next_chunk(&mut self) -> Result<(Vec<AtomicToken>, Option<ChunkExtraInfo>), Error> {
        self.fill_buffer_until_n_chunks(2)?;

        // prevent creating too small chunk
        let next_chunk_size = if self.config.chunk_size < self.curr_buffer_size && self.curr_buffer_size < self.config.chunk_size * 2 {
            self.curr_buffer_size / 2
        } else {
            self.config.chunk_size
        };

        let mut chunk_deque = VecDeque::new();
        let mut curr_chunk_size = 0;
        let mut has_page_break = false;
        let mut chunk_extra_info: Option<ChunkExtraInfo> = None;

        // step 1. collect tokens for a chunk
        while curr_chunk_size < next_chunk_size && !self.buffer.is_empty() {
            let token = self.buffer.pop_front().unwrap();

            if let AtomicToken::ChunkExtraInfo(extra_info) = &token {
                match &mut chunk_extra_info {
                    Some(old) => {
                        *old = old.merge(extra_info);
                    },
                    None => {
                        chunk_extra_info = Some(extra_info.clone());
                    },
                }

                continue;
            }

            if let AtomicToken::PageBreak = &token {
                has_page_break = true;
                break;
            }

            self.curr_buffer_size -= token.len(self.config.image_size);
            curr_chunk_size += token.len(self.config.image_size);
            chunk_deque.push_back(token);
        }

        // step 2. create a sliding window
        // if there's no remaining token, there's no need for sliding window
        // if the chunk consists of a single token, there's no point in making a sliding window
        if !has_page_break && (!self.buffer.is_empty() || chunk_deque.len() == 1) {
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

        Ok((tokens, chunk_extra_info))
    }

    pub async fn generate_chunk(
        &mut self,
        index: &Index,
        build_info: ChunkBuildInfo,
        previous_turn: Option<(Chunk, ChunkSchema)>,
        index_in_file: usize,
    ) -> Result<Chunk, Error> {
        let (tokens, chunk_extra_info) = self.next_chunk()?;
        let tokens = self.fetch_images_from_web(tokens).await?;

        let chunk = Chunk::create_chunk_from(
            index,
            &tokens,
            self.rel_path.clone(),
            index_in_file,
            build_info,
            previous_turn,
            chunk_extra_info,
        ).await;

        if let Some(ms) = index.api_config.sleep_after_llm_call {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        chunk
    }

    fn fill_buffer_until_n_chunks(&mut self, n: usize) -> Result<(), Error> {
        loop {
            if self.curr_buffer_size >= n * self.config.chunk_size {
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

    /// It replaces `AtomicToken::WebImage` in `tokens` with `AtomicToken::Image`.
    async fn fetch_images_from_web(&mut self, tokens: Vec<AtomicToken>) -> Result<Vec<AtomicToken>, Error> {
        let mut new_tokens = Vec::with_capacity(tokens.len());

        for token in tokens.into_iter() {
            match &token {
                AtomicToken::WebImage { url, .. } => {
                    if let Some(uid) = self.fetched_images.get(url) {
                        let bytes = self.images.get(uid).unwrap();

                        new_tokens.push(AtomicToken::Image(Image {
                            bytes: bytes.to_vec(),
                            image_type: ImageType::Png,  // It's already normalized
                            uid: *uid,
                        }));
                    }

                    else {
                        match fetch_image_from_web(url).await {
                            Ok((bytes, image_type)) => {
                                let image = Image::new(bytes, image_type)?;
                                self.images.insert(image.uid, image.bytes.clone());
                                self.fetched_images.insert(url.to_string(), image.uid);

                                new_tokens.push(AtomicToken::Image(image));
                            },
                            Err(e) => if self.config.strict_file_reader {
                                return Err(e);
                            } else {
                                new_tokens.push(token);
                            },
                        }
                    }
                },
                _ => { new_tokens.push(token); },
            }
        }

        Ok(new_tokens)
    }
}

fn merge_tokens(tokens: VecDeque<AtomicToken>) -> Vec<AtomicToken> {
    let mut buffer = vec![];
    let mut result = vec![];

    for token in tokens.into_iter() {
        match token {
            AtomicToken::String { data, .. } => { buffer.push(data); },
            AtomicToken::Image(_) | AtomicToken::WebImage { .. } => {
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
            AtomicToken::PageBreak
           | AtomicToken::ChunkExtraInfo(_) => {},
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

async fn fetch_image_from_web(url: &str) -> Result<(Vec<u8>, ImageType), Error> {
    let image_type = ImageType::from_extension(&extension(url)?.unwrap_or(String::new()))?;
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    let status_code = response.status().as_u16();

    if status_code != 200 {
        return Err(Error::FileReaderError(format!("GET {url} returned {status_code}.")));
    }

    let bytes = response.bytes().await?.to_vec();
    Ok((bytes, image_type))
}

#[derive(Clone, Debug)]
pub enum AtomicToken {
    String {
        data: String,
        char_len: usize,
    },
    Image(Image),

    /// You can fetch images from web!
    ///
    /// If your file reader generates this token, ragit will
    /// handle it. If it fails to fetch the image, 1) if it's
    /// strict mode, it will crash. 2) Otherwise, it would
    /// create a string token with `subst`.
    WebImage { url: String, subst: String },

    /// It's an invisible AtomicToken.
    /// An AtomicToken before a break and after the
    /// break will never belong to the same chunk.
    PageBreak,

    /// You can add extra information to the chunk.
    ///
    /// I recommend you use `ChunkExtraInfo` right before
    /// `PageBreak`, so that each chunk has at most
    /// 1 extra information.
    ///
    /// If there are multiple `ChunkExtraInfo`s in a chunk,
    /// ragit will do *its best* to interpret them.
    ChunkExtraInfo(ChunkExtraInfo),
}

impl AtomicToken {
    pub fn len(&self, image_size: usize) -> usize {
        match self {
            AtomicToken::String { char_len, .. } => *char_len,
            AtomicToken::Image(_) => image_size,
            AtomicToken::WebImage { .. } => image_size,
            AtomicToken::PageBreak
           | AtomicToken::ChunkExtraInfo(_) => 0,
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

            // This variant is supposed to be removed by `FileReader::generate_chunk`.
            // If this branch is reached, that means it's failed to fetch the image.
            AtomicToken::WebImage { subst, url: _ } => MessageContent::String(subst.clone()),

            // this branch is not supposed to be reached
            AtomicToken::PageBreak
           | AtomicToken::ChunkExtraInfo(_) => MessageContent::String(String::new()),
        }
    }
}
