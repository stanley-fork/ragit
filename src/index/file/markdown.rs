use super::{AtomicToken, FileReaderImpl, Image};
use crate::error::Error;
use crate::index::Config;
use ragit_api::ImageType;
use ragit_fs::{FileError, exists, extension, join, parent, read_bytes};
use regex::Regex;
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};

pub struct MarkdownReader {
    path: String,
    lines: BufReader<std::fs::File>,
    tokens: Vec<AtomicToken>,
    is_exhausted: bool,
    strict_mode: bool,
    curr_parse_state: ParseState,
    link_reference_definitions: HashMap<String, String>,
}

impl FileReaderImpl for MarkdownReader {
    fn new(path: &str, config: &Config) -> Result<Self, Error> {
        match std::fs::File::open(path) {
            Ok(f) => Ok(MarkdownReader {
                path: path.to_string(),
                lines: BufReader::new(f),
                tokens: vec![],
                is_exhausted: false,
                strict_mode: config.strict_file_reader,
                curr_parse_state: ParseState::Paragraph,
                link_reference_definitions: HashMap::new(),
            }),
            Err(e) => Err(FileError::from_std(e, path).into()),
        }
    }

    fn load_tokens(&mut self) -> Result<(), Error> {
        if self.is_exhausted {
            return Ok(());
        }

        let mut buffer = vec![];

        loop {
            // NOTE: `line` includes a newline character
            let mut line = String::new();

            if self.lines.read_line(&mut line)? == 0 {
                self.is_exhausted = true;
                self.consume_buffer(buffer)?;
                break;
            }

            if buffer.len() > 16 && !has_unknown_link_reference(&self.link_reference_definitions, &buffer) {
                self.consume_buffer(buffer)?;
                break;
            }

            match &self.curr_parse_state {
                ParseState::Paragraph => match parse_code_fence(&line) {
                    Some(fence) => {
                        self.curr_parse_state = ParseState::CodeFence(fence);
                    },
                    None => {
                        if let Some((label, destination)) = parse_link_reference_definition(&line) {
                            self.link_reference_definitions.insert(label, destination);
                            continue;
                        }

                        for token in parse_markdown_images(&line)? {
                            buffer.push(token);
                        }

                        continue;
                    },
                },
                ParseState::CodeFence(fence) => match parse_code_fence(&line) {
                    Some(fence2) => {
                        if match_fences(fence, &fence2) {
                            self.curr_parse_state = ParseState::Paragraph;
                        }
                    },
                    None => {},
                },
            }

            buffer.push(StringOrImage::String(line));
        }

        Ok(())
    }

    fn pop_all_tokens(&mut self) -> Result<Vec<AtomicToken>, Error> {
        let mut result = vec![];
        std::mem::swap(&mut self.tokens, &mut result);
        Ok(result)
    }

    fn has_more_to_read(&self) -> bool {
        !self.is_exhausted
    }

    fn key(&self) -> String {
        String::from("markdown_reader_v0")
    }
}

impl MarkdownReader {
    fn consume_buffer(&mut self, buffer: Vec<StringOrImage>) -> Result<(), Error> {
        for token in buffer.into_iter() {
            match token {
                StringOrImage::String(s) => {
                    self.tokens.push(AtomicToken::String {
                        char_len: s.chars().count(),
                        data: s,
                    });
                },
                _ => {
                    let (desc, mut url) = match token {
                        StringOrImage::ImageUrl { desc, url } => (desc, url),
                        StringOrImage::ImageRef { desc, r#ref } => match self.link_reference_definitions.get(&r#ref) {
                            Some(url) => (desc, url.to_string()),
                            _ => {
                                if self.strict_mode {
                                    return Err(Error::FileReaderError(format!("Cannot find image link reference: {ref:?}")));
                                }

                                let fallback = format!("![{desc}][{ref}]");
                                self.tokens.push(AtomicToken::String {
                                    char_len: fallback.chars().count(),
                                    data: fallback,
                                });
                                continue;
                            },
                        },
                        _ => unreachable!(),
                    };

                    if !exists(&url) {
                        if let Ok(joined_url) = join(&parent(&self.path)?, &url) {
                            url = joined_url;
                        }
                    }

                    let bytes = match read_bytes(&url) {
                        Ok(bytes) => bytes,
                        Err(e) => if self.strict_mode {
                            return Err(e.into());
                        } else {
                            let fallback = format!("![{desc}](url)");
                            self.tokens.push(AtomicToken::String {
                                char_len: fallback.chars().count(),
                                data: fallback,
                            });
                            continue;
                        },
                    };
                    let mut hasher = Sha3_256::new();
                    hasher.update(&bytes);

                    self.tokens.push(AtomicToken::Image(Image {
                        image_type: ImageType::from_extension(&extension(&url).unwrap_or(Some(String::from("png"))).unwrap_or(String::from("png"))).unwrap_or(ImageType::Png),
                        bytes,
                        key: format!("{:064x}", hasher.finalize()),
                    }));
                },
            }
        }

        Ok(())
    }
}

enum ParseState {
    Paragraph,
    CodeFence(CodeFence),
}

struct CodeFence {
    fence_char: u8,  // ` or ~
    fence_len: usize,
    info_string: Option<String>,
    indent: usize,
}

#[derive(Clone)]
enum StringOrImage {
    String(String),
    ImageUrl { desc: String, url: String },    // ![desc](url)
    ImageRef { desc: String, r#ref: String },  // ![ref] or ![desc][ref]
}

// https://github.github.com/gfm/#fenced-code-blocks
fn parse_code_fence(line: &str) -> Option<CodeFence> {
    let fence_re = Regex::new(r"(\s*)(\`{3,}|\~{3,})([^`]*)").unwrap();
    fence_re.captures(line).map(
        |cap| {
            let indent = cap[1].len();
            let fence = cap[2].to_string();
            let info_string = cap[3].trim().to_string();

            CodeFence {
                fence_char: fence.as_bytes()[0],
                fence_len: fence.len(),
                info_string: if info_string.is_empty() { None } else { Some(info_string) },
                indent,
            }
        }
    )
}

fn match_fences(start: &CodeFence, end: &CodeFence) -> bool {
    start.fence_char == end.fence_char &&
    start.fence_len <= end.fence_len &&
    end.indent < 4 &&
    end.info_string.is_none()
}

// https://github.github.com/gfm/#link-reference-definition
// TODO: it cannot handle multi-line link reference definitions
fn parse_link_reference_definition(line: &str) -> Option<(String, String)> {
    let def_re = Regex::new(r"\s{0,3}\[([^\[\]]{1,999})\]\s?\:\s?(.+)").unwrap();
    let result = def_re.captures(line).map(
        |cap| (
            normalize_link_label(&cap[1]),
            cap[2].trim().to_string(),
        )
    );

    if let Some((label, _)) = &result {
        if label.is_empty() { return None; }
    }

    result
}

fn normalize_link_label(label: &str) -> String {
    let label = label.trim().to_lowercase();
    let label = label.replace("\n", " ");
    let label = label.replace("\t", " ");
    let label = label.replace("\r", " ");
    let mut label = label.replace("  ", " ");

    while label.contains("  ") {
        label = label.replace("  ", " ");
    }

    label
}

fn has_unknown_link_reference(
    link_reference_definitions: &HashMap<String, String>,
    buffer: &[StringOrImage],
) -> bool {
    for token in buffer.iter() {
        if let StringOrImage::ImageRef { r#ref, .. } = token {
            if !link_reference_definitions.contains_key(r#ref) {
                return true;
            }
        }
    }

    false
}

// NOTE: It's a quite dumb parser. It cannot handle some edge cases, but that won't be a problem in perspective of RAG.
fn parse_markdown_images(line: &str) -> Result<Vec<StringOrImage>, Error> {
    let image_re = Regex::new(r"(?s)^(.*?)!\[([^\[\]]+)\](.*)$").unwrap();

    if let Some(cap) = image_re.captures(line) {
        let pre = cap[1].to_string();
        let label = cap[2].to_string();
        let mut post = cap[3].to_string();
        let mut result = vec![];

        if !pre.is_empty() {
            result.push(StringOrImage::String(pre));
        }

        let paren_re = Regex::new(r"(?s)^\(([^()]*)\)(.*)$").unwrap();
        let bracket_re = Regex::new(r"(?s)^\[([^\[\]]*)\](.*)$").unwrap();

        if let Some(cap) = paren_re.captures(&post) {
            result.push(StringOrImage::ImageUrl {
                desc: label,
                url: cap[1].trim().to_string(),
            });
            post = cap[2].to_string();
        }

        else if let Some(cap) = bracket_re.captures(&post) {
            let r#ref = normalize_link_label(&cap[1]);

            if r#ref.is_empty() {
                result.push(StringOrImage::ImageRef {
                    desc: String::new(),
                    r#ref: normalize_link_label(&label),
                });
            }

            else {
                result.push(StringOrImage::ImageRef {
                    desc: label,
                    r#ref,
                });
            }

            post = cap[2].to_string();
        }

        else {
            result.push(StringOrImage::ImageRef {
                desc: String::new(),
                r#ref: normalize_link_label(&label),
            });
        }

        if !post.is_empty() {
            result = vec![
                result.clone(),
                parse_markdown_images(&post)?,
            ].concat();
        }

        Ok(result)
    }

    else {
        Ok(vec![StringOrImage::String(line.to_string())])
    }
}
