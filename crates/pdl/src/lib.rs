use lazy_static::lazy_static;
use ragit_fs::{extension, join, parent, read_bytes, read_string};
use regex::bytes::Regex;

mod error;
mod image;
mod message;
mod role;
mod schema;
mod util;

pub use error::Error;
pub use image::ImageType;
pub use message::{Message, MessageContent};
pub use role::{PdlRole, Role};
pub use schema::{Schema, SchemaParseError, parse_schema};
pub use util::{decode_base64, encode_base64};

lazy_static! {
    static ref MEDIA_RE: Regex = Regex::new(r"^media\((.+)\)$").unwrap();
    static ref RAW_MEDIA_RE: Regex = Regex::new(r"^raw_media\(([a-zA-Z0-9]+):([^:]+)\)$").unwrap();
}

#[derive(Clone, Debug)]
pub struct Pdl {
    pub schema: Option<Schema>,
    pub messages: Vec<Message>,
}

impl Pdl {
    pub fn validate(&self) -> Result<(), Error> {
        if self.messages.is_empty() {
            return Err(Error::InvalidPdl(String::from("A pdl file is empty.")));
        }

        let mut after_user = false;
        let mut after_assistant = false;

        for (index, Message { role, .. }) in self.messages.iter().enumerate() {
            match role {
                Role::User => {
                    if after_user {
                        return Err(Error::InvalidPdl(String::from("<|user|> appeared twice in a row.")));
                    }

                    after_user = true;
                    after_assistant = false;
                },
                Role::Assistant => {
                    if after_assistant {
                        return Err(Error::InvalidPdl(String::from("<|assistant|> appeared twice in a row.")));
                    }

                    after_user = false;
                    after_assistant = true;
                },
                Role::System => {
                    if index != 0 {
                        return Err(Error::InvalidPdl(String::from("<|system|> must appear at top.")));
                    }
                },
                Role::Reasoning => {},  // TODO
            }
        }

        match self.messages.last() {
            Some(Message { role: Role::Assistant, .. }) => {
                return Err(Error::InvalidPdl(String::from("A pdl file ends with <|assistant|>.")));
            },
            _ => {},
        }

        Ok(())
    }
}

pub fn parse_pdl_from_file(
    path: &str,
    context: &tera::Context,

    // If it's not set, it would never return `Err`.
    strict_mode: bool,

    // If it's set, it unescapes characters in `s`.
    is_escaped: bool,
) -> Result<Pdl, Error> {
    parse_pdl(
        &read_string(path)?,
        context,
        &parent(path)?,
        strict_mode,
        is_escaped,
    )
}

pub fn parse_pdl(
    s: &str,
    context: &tera::Context,
    curr_dir: &str,

    // If it's not set, it would never return `Err`.
    strict_mode: bool,

    // If it's set, it unescapes characters in `s`.
    is_escaped: bool,
) -> Result<Pdl, Error> {
    let tera_rendered = match tera::Tera::one_off(s, context, true) {
        Ok(t) => t,
        Err(e) => if strict_mode {
            return Err(e.into());
        } else {
            s.to_string()
        },
    };

    let mut messages = vec![];
    let mut schema = None;
    let mut curr_role = None;
    let mut line_buffer = vec![];

    // simple hack: Adding this line to the content makes the code
    // handle the last turn correctly. Since this fake turn is empty,
    // it will be removed later.
    let last_line = "<|assistant|>";

    for line in tera_rendered.lines().chain(std::iter::once(last_line)) {
        let trimmed = line.trim();

        // maybe a turn-separator
        if trimmed.starts_with("<|") && trimmed.ends_with("|>") && trimmed.len() > 4 {
            match trimmed.to_ascii_lowercase().get(2..(trimmed.len() - 2)).unwrap() {
                t @ ("user" | "system" | "assistant" | "schema" | "reasoning") => {
                    if !line_buffer.is_empty() || curr_role.is_some() {
                        match curr_role {
                            Some(PdlRole::Schema) => match parse_schema(&line_buffer.join("\n")) {
                                Ok(s) => {
                                    if schema.is_some() && strict_mode {
                                        return Err(Error::InvalidPdl(String::from("<|schema|> appeared multiple times.")));
                                    }

                                    schema = Some(s);
                                },
                                Err(e) => {
                                    if strict_mode {
                                        return Err(e.into());
                                    }
                                },
                            },
                            // reasoning tokens are not fed to llm contexts
                            Some(PdlRole::Reasoning) => {},
                            _ => {
                                // there must be lots of unnecessary newlines due to the nature of the format
                                // let's just trim them away
                                let raw_contents = line_buffer.join("\n");
                                let raw_contents = raw_contents.trim();

                                let role = match curr_role {
                                    Some(role) => role,
                                    None => {
                                        if raw_contents.is_empty() {
                                            curr_role = Some(PdlRole::from(t));
                                            line_buffer = vec![];
                                            continue;
                                        }

                                        if strict_mode {
                                            return Err(Error::RoleMissing);
                                        }

                                        PdlRole::System
                                    },
                                };

                                match into_message_contents(&raw_contents, is_escaped, curr_dir) {
                                    Ok(t) => {
                                        messages.push(Message {
                                            role: role.into(),
                                            content: t,
                                        });
                                    },
                                    Err(e) => {
                                        if strict_mode {
                                            return Err(e);
                                        }

                                        else {
                                            messages.push(Message {
                                                role: role.into(),
                                                content: vec![MessageContent::String(raw_contents.to_string())],
                                            });
                                        }
                                    },
                                }
                            },
                        }
                    }

                    curr_role = Some(PdlRole::from(t));
                    line_buffer = vec![];
                    continue;
                },
                t => {
                    if strict_mode && t.chars().all(|c| c.is_ascii_alphabetic()) {
                        return Err(Error::InvalidTurnSeparator(t.to_string()));
                    }

                    line_buffer.push(line.to_string());
                },
            }
        }

        else {
            line_buffer.push(line.to_string());
        }
    }

    if let Some(Message { content, .. }) = messages.last() {
        if content.is_empty() {
            messages.pop().unwrap();
        }
    }

    let result = Pdl {
        schema,
        messages,
    };

    if strict_mode {
        result.validate()?;
    }

    Ok(result)
}

pub fn escape_pdl_tokens(s: &str) -> String {  // TODO: use `Cow` type
    s.replace("&", "&amp;").replace("<|", "&lt;|")
}

pub fn unescape_pdl_tokens(s: &str) -> String {  // TODO: use `Cow` type
    s.replace("&lt;", "<").replace("&amp;", "&")
}

fn into_message_contents(s: &str, is_escaped: bool, curr_dir: &str) -> Result<Vec<MessageContent>, Error> {
    let bytes = s.as_bytes().iter().map(|b| *b).collect::<Vec<_>>();
    let mut index = 0;
    let mut result = vec![];
    let mut string_buffer = vec![];

    loop {
        match bytes.get(index) {
            Some(b'<') => match try_parse_inline_block(&bytes, index, curr_dir) {
                Ok(Some((image_type, bytes, new_index))) => {
                    if !string_buffer.is_empty() {
                        match String::from_utf8(string_buffer.clone()) {
                            Ok(s) => {
                                if is_escaped {
                                    result.push(MessageContent::String(unescape_pdl_tokens(&s)));
                                }

                                else {
                                    result.push(MessageContent::String(s));
                                }
                            },
                            Err(e) => {
                                return Err(e.into());
                            },
                        }
                    }

                    result.push(MessageContent::Image { image_type, bytes });
                    index = new_index;
                    string_buffer = vec![];
                    continue;
                },
                Ok(None) => {
                    string_buffer.push(b'<');
                },
                Err(e) => {
                    return Err(e);
                },
            },
            Some(b) => {
                string_buffer.push(*b);
            },
            None => {
                if !string_buffer.is_empty() {
                    match String::from_utf8(string_buffer) {
                        Ok(s) => {
                            if is_escaped {
                                result.push(MessageContent::String(unescape_pdl_tokens(&s)));
                            }

                            else {
                                result.push(MessageContent::String(s));
                            }
                        },
                        Err(e) => {
                            return Err(e.into());
                        },
                    }
                }

                break;
            },
        }

        index += 1;
    }

    Ok(result)
}

// 1. It returns `Ok(Some(_))` if it's a valid inline block.
// 2. It returns `Ok(None)` if it's not an inline block.
// 3. It returns `Err(_)` if it's an inline block, but there's an error (syntax error, image type error, file error, ...).
fn try_parse_inline_block(bytes: &[u8], index: usize, curr_dir: &str) -> Result<Option<(ImageType, Vec<u8>, usize)>, Error> {
    match try_get_pdl_token(bytes, index) {
        Some((token, new_index)) => {
            let media_re = &MEDIA_RE;
            let raw_media_re = &RAW_MEDIA_RE;

            if let Some(cap) = raw_media_re.captures(token) {
                let image_type = String::from_utf8_lossy(&cap[1]).to_string();
                let image_bytes = String::from_utf8_lossy(&cap[2]).to_string();

                Ok(Some((ImageType::from_extension(&image_type)?, decode_base64(&image_bytes)?, new_index)))
            }

            else if let Some(cap) = media_re.captures(token) {
                let path = &cap[1];
                let file = join(curr_dir, &String::from_utf8_lossy(path).to_string())?;
                Ok(Some((ImageType::from_extension(&extension(&file)?.unwrap_or(String::new()))?, read_bytes(&file)?, new_index)))
            }

            else {
                Err(Error::InvalidInlineBlock)
            }
        },

        // not an inline block at all
        None => Ok(None),
    }
}

fn try_get_pdl_token(bytes: &[u8], mut index: usize) -> Option<(&[u8], usize)> {
    let old_index = index;

    match (bytes.get(index), bytes.get(index + 1)) {
        (Some(b'<'), Some(b'|')) => {
            index += 2;

            loop {
                match (bytes.get(index), bytes.get(index + 1)) {
                    (Some(b'|'), Some(b'>')) => {
                        return Some((&bytes[(old_index + 2)..index], index + 2));
                    },
                    (_, Some(b'|')) => {
                        index += 1;
                    },
                    (_, None) => {
                        return None;
                    },
                    _ => {
                        index += 2;
                    },
                }
            }
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ImageType,
        Message,
        MessageContent,
        Pdl,
        Role,
        decode_base64,
        parse_pdl,
        parse_pdl_from_file,
    };
    use ragit_fs::{
        WriteMode,
        join,
        temp_dir,
        write_string,
    };

    // more thorough test suites are in `tests/`
    #[test]
    fn messages_from_file_test() {
        let tmp_path = join(
            &temp_dir().unwrap(),
            "test_messages.tera",
        ).unwrap();

        write_string(
            &tmp_path,
"
<|system|>

You're a code helper.

<|user|>

Write me a sudoku-solver.


",
            WriteMode::CreateOrTruncate,
        ).unwrap();

        let Pdl { messages, schema } = parse_pdl_from_file(
            &tmp_path,
            &tera::Context::new(),
            true,
            true,
        ).unwrap();

        assert_eq!(
            messages,
            vec![
                Message {
                    role: Role::System,
                    content: vec![
                        MessageContent::String(String::from("You're a code helper.")),
                    ],
                },
                Message {
                    role: Role::User,
                    content: vec![
                        MessageContent::String(String::from("Write me a sudoku-solver.")),
                    ],
                },
            ],
        );
        assert_eq!(
            schema,
            None,
        );
    }

    #[test]
    fn media_content_test() {
        let Pdl { messages, schema } = parse_pdl(
"
<|user|>

<|raw_media(png:HiMyNameIsBaehyunsol)|>
",
            &tera::Context::new(),
            ".",  // there's no `<|media|>`
            true,
            true,
        ).unwrap();

        assert_eq!(
            messages,
            vec![
                Message {
                    role: Role::User,
                    content: vec![
                        MessageContent::Image {
                            image_type: ImageType::Png,
                            bytes: decode_base64("HiMyNameIsBaehyunsol").unwrap(),
                        },
                    ],
                },
            ],
        );
        assert_eq!(
            schema,
            None,
        );
    }
}
