use super::{AtomicToken, FileReaderImpl, Image};
use crate::error::Error;
use crate::index::BuildConfig;
use crate::uid::Uid;
use ragit_api::ImageType;
use ragit_fs::{FileError, exists, extension, join, parent, read_bytes};
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct MarkdownReader {
    path: String,
    lines: BufReader<File>,
    tokens: Vec<AtomicToken>,
    is_exhausted: bool,
    strict_mode: bool,
    curr_parse_state: ParseState,
    link_reference_definitions: HashMap<String, String>,
}

impl FileReaderImpl for MarkdownReader {
    fn new(path: &str, config: &BuildConfig) -> Result<Self, Error> {
        match File::open(path) {
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
                    let uid = Uid::new_image(&bytes);
                    self.tokens.push(AtomicToken::Image(Image {
                        image_type: ImageType::from_extension(&extension(&url).unwrap_or(Some(String::from("png"))).unwrap_or(String::from("png"))).unwrap_or(ImageType::Png),
                        bytes,
                        uid,
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

#[derive(Clone, Debug)]
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

fn parse_markdown_images(line: &str) -> Result<Vec<StringOrImage>, Error> {
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut last_index = 0;
    let mut result = vec![];

    while index < chars.len() {
        if is_code_span_start(&chars, index) {
            index = march_until_code_span_end(&chars, index);
        }

        else {
            match try_parse_image(&chars, index) {
                Some(image) => {
                    if last_index < index {
                        result.push(StringOrImage::String(chars[last_index..index].iter().collect()));
                    }

                    index = march_until_image_end(&chars, index);
                    last_index = index;
                    result.push(image);
                },
                None => {
                    index += 1;
                    index = march_until_important_char(&chars, index);
                },
            }
        }
    }

    if last_index < index {
        result.push(StringOrImage::String(chars[last_index..index].iter().collect()));
    }

    Ok(result)
}

fn is_code_span_start(chars: &[char], index: usize) -> bool {
    matches!(chars.get(index), Some('`')) && chars.len() > index + 1 && chars[index..].iter().any(|c| *c != '`')
}

// It assumes that `is_code_span_start(chars, index)` is true.
// It returns the index of the first character after the code span. -> one that comes after '`'
// If the code span does not end (probably a markdown syntax error), it returns the index of the last character.
fn march_until_code_span_end(chars: &[char], index: usize) -> usize {
    let mut backtick_count = 0;
    let original_len = chars.len();
    let chars = &chars[index..];

    for (i, c) in chars.iter().enumerate() {
        if *c != '`' {
            backtick_count = i;
            break;
        }
    }

    assert!(backtick_count != 0);

    for i in 1..(chars.len() - backtick_count) {
        if &chars[i..(i + backtick_count)] == &vec!['`'; backtick_count] {
            return index + i + backtick_count;
        }
    }

    return original_len - 1;
}

fn try_parse_image(chars: &[char], index: usize) -> Option<StringOrImage> {
    match chars.get(index) {
        Some('!') => match chars.get(index + 1) {
            Some('[') => {},
            _ => {
                return None;
            },
        },
        _ => {
            return None;
        },
    }

    let (bracket_content, index) = match get_matching_bracket_index(chars, index + 1) {
        Some(new_index) => (chars[index + 2..new_index].iter().collect::<String>(), new_index),
        None => {
            return None;
        },
    };

    match chars.get(index + 1) {
        Some('[') => match get_matching_bracket_index(chars, index + 1) {
            Some(new_index) => {
                let r#ref = normalize_link_label(&chars[index + 2..new_index].iter().collect::<String>());

                if r#ref.is_empty() {
                    return None;
                }

                return Some(StringOrImage::ImageRef { desc: bracket_content, r#ref });
            },
            None => {},
        },
        Some('(') => match get_matching_bracket_index(chars, index + 1) {
            Some(new_index) => {
                return Some(StringOrImage::ImageUrl {
                    desc: bracket_content,
                    url: chars[index + 2..new_index].iter().collect::<String>(),
                });
            },
            None => {},
        },
        _ => {},
    }

    let r#ref = normalize_link_label(&bracket_content);

    if r#ref.is_empty() {
        return None;
    }

    Some(StringOrImage::ImageRef { desc: String::new(), r#ref })
}

// It assumes that `try_parse_image(chars, index).is_some()`.
fn march_until_image_end(chars: &[char], index: usize) -> usize {
    let index = get_matching_bracket_index(chars, index + 1).unwrap();

    match chars.get(index + 1) {
        Some('[' | '(') => match get_matching_bracket_index(chars, index + 1) {
            Some(index) => index + 1,
            None => index + 1,
        },
        _ => index + 1,
    }
}

fn march_until_important_char(chars: &[char], index: usize) -> usize {
    for i in index.. {
        match chars.get(i) {
            Some(c) if *c == '`' || *c == '!' => {
                return i;
            },
            None => {
                return i;
            },
            _ => {},
        }
    }

    unreachable!()
}

fn get_matching_bracket_index(chars: &[char], mut index: usize) -> Option<usize> {
    let end = match chars.get(index) {
        Some('[') => ']',
        Some('(') => ')',
        Some('{') => '}',
        _ => {
            return None;
        },
    };
    index += 1;

    loop {
        match chars.get(index) {
            Some(c) if *c == end => {
                return Some(index);
            },
            Some('(' | '[' | '{') => match get_matching_bracket_index(chars, index) {
                Some(new_index) => {
                    index = new_index + 1;
                },
                _ => {
                    return None;
                },
            },
            Some(')' | ']' | '}') => {
                return None;
            },
            None => {
                return None;
            },
            _ => {
                index += 1;
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{AtomicToken, FileReaderImpl};
    use super::MarkdownReader;
    use crate::index::BuildConfig;
    use ragit_fs::{WriteMode, remove_file, write_string};

    #[test]
    fn markdown_test() {
        let config_default = BuildConfig::default();
        let mut config_strict = config_default.clone();
        config_strict.strict_file_reader = true;
        let md1 = "
# Title

This is a markdown file that has no image.

![This is a broken image
";
        write_string("__tmp_test.md", md1, WriteMode::AlwaysCreate).unwrap();
        let mut md_reader = MarkdownReader::new("__tmp_test.md", &config_strict).unwrap();

        while md_reader.has_more_to_read() {
            md_reader.load_tokens().unwrap();
        }

        let md1_tokens = md_reader.pop_all_tokens().unwrap();
        assert_eq!(
            md1_tokens.iter().map(
                |token| match token {
                    AtomicToken::String { data, .. } => data.to_string(),
                    _ => panic!(),
                }
            ).collect::<Vec<_>>().concat(),
            md1.to_string(),
        );
        remove_file("__tmp_test.md").unwrap();
    }
}
