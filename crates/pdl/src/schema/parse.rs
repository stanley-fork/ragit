use super::{Constraint, Schema, SchemaType};
use std::fmt;

#[derive(Debug)]
pub enum SchemaParseError {
    UnexpectedByte(u8),
    UnmatchedGroup(u8),  // an opening delim
    UnexpectedToken(Token),
    UnexpectedEof,
    ParseFloatError(std::num::ParseFloatError),
    Utf8Error(std::string::FromUtf8Error),
    InvalidConstraint(String),
}

#[derive(Clone, Debug)]
pub enum Token {
    Literal(String),
    Integer(i64),
    Float(f64),
    Group {
        kind: GroupKind,
        tokens: Vec<Token>,
    },

    /// ':' | ','
    Punct(u8),
}

impl fmt::Display for Token {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Token::Literal(s) => write!(fmt, "{s:?}"),
            Token::Integer(n) => write!(fmt, "{n:?}"),
            Token::Float(n) => write!(fmt, "{n:?}"),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum GroupKind {
    Brace,
    Parenthesis,
    Bracket,
}

impl From<u8> for GroupKind {
    fn from(c: u8) -> Self {
        match c {
            b'{' | b'}' => GroupKind::Brace,
            b'(' | b')' => GroupKind::Parenthesis,
            b'[' | b']' => GroupKind::Bracket,
            _ => unreachable!(),
        }
    }
}

enum TokenizeState {
    Init,
    Number,
    Identifier,
    Literal(u8),
}

pub fn parse_schema(s: &str) -> Result<Schema, SchemaParseError> {
    let mut index = 0;
    let s = s.as_bytes();
    let tokens = tokenize(s, &mut index)?;

    if let Some(b) = s.get(index) {
        return Err(SchemaParseError::UnexpectedByte(*b));
    }

    let mut index = 0;
    let result = token_to_schema(&tokens, &mut index)?;
    result.validate_constraint()?;

    Ok(result)
}

fn tokenize(s: &[u8], index: &mut usize) -> Result<Vec<Token>, SchemaParseError> {
    let mut curr_state = TokenizeState::Init;
    let mut result = vec![];
    let mut cursor = *index;

    loop {
        match curr_state {
            TokenizeState::Init => match s.get(*index) {
                Some(d @ (b'{' | b'(' | b'[')) => {
                    *index += 1;
                    let inner = tokenize(s, index)?;

                    if s.get(*index) == Some(&matching_delim(*d)) {
                        result.push(Token::Group {
                            kind: GroupKind::from(*d),
                            tokens: inner,
                        });
                    }

                    else {
                        return Err(SchemaParseError::UnmatchedGroup(*d));
                    }
                },
                Some(b'}' | b')' | b']') => {
                    return Ok(result);
                },
                Some(m @ (b'"' | b'\'')) => {
                    curr_state = TokenizeState::Literal(*m);
                    cursor = *index + 1;
                },
                Some(b'0'..=b'9') => {
                    curr_state = TokenizeState::Number;
                    cursor = *index;
                },
                Some(b' ' | b'\n' | b'\r' | b'\t') => {},
                Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'-') => {
                    curr_state = TokenizeState::Identifier;
                    cursor = *index;
                },
                Some(p @ (b':' | b',')) => {
                    result.push(Token::Punct(*p));
                },
                Some(c) => {
                    return Err(SchemaParseError::UnexpectedByte(*c));
                },
                None => {
                    return Ok(result);
                },
            },
            TokenizeState::Number => match s.get(*index) {
                Some(b'0'..=b'9' | b'.') => {},
                _ => {
                    let ns = String::from_utf8_lossy(&s[cursor..*index]).to_string();

                    match ns.parse::<i64>() {
                        Ok(n) => {
                            curr_state = TokenizeState::Init;
                            result.push(Token::Integer(n));
                            continue;
                        },
                        Err(_) => match ns.parse::<f64>() {
                            Ok(n) => {
                                curr_state = TokenizeState::Init;
                                result.push(Token::Float(n));
                                continue;
                            },
                            Err(e) => {
                                return Err(SchemaParseError::ParseFloatError(e));
                            },
                        },
                    }
                },
            },
            TokenizeState::Identifier => match s.get(*index) {
                Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'-') => {},
                _ => match String::from_utf8(s[cursor..*index].to_vec()) {
                    Ok(s) => {
                        curr_state = TokenizeState::Init;
                        result.push(Token::Literal(s));
                        continue;
                    },
                    Err(e) => {
                        return Err(SchemaParseError::Utf8Error(e));
                    },
                },
            },
            TokenizeState::Literal(marker) => match s.get(*index) {
                Some(c) if *c == marker => match String::from_utf8(s[cursor..*index].to_vec()) {
                    Ok(s) => {
                        curr_state = TokenizeState::Init;
                        result.push(Token::Literal(s));
                        continue;
                    },
                    Err(e) => {
                        return Err(SchemaParseError::Utf8Error(e));
                    },
                },
                Some(_) => {},
                None => {
                    return Err(SchemaParseError::UnmatchedGroup(marker));
                },
            },
        }

        *index += 1;
    }
}

fn token_to_schema(tokens: &[Token], index: &mut usize) -> Result<Schema, SchemaParseError> {
    let mut r#type = match tokens.get(*index) {
        Some(t @ Token::Literal(s)) => match s.as_str() {
            "str" | "string" => Schema::default_string(),
            "int" | "integer" => Schema::default_integer(),
            "float" | "number" => Schema::default_float(),
            "bool" | "boolean" => Schema::default_boolean(),
            "yesno" => Schema::default_yesno(),
            "code" => Schema::default_code(),
            "tasklist" => Schema::default_task_list(),
            _ => {
                return Err(SchemaParseError::UnexpectedToken(t.clone()));
            },
        },
        Some(Token::Group {
            kind: GroupKind::Brace,
            tokens: inner,
        }) => {
            let mut inner_index = 0;
            let mut result = vec![];

            loop {
                let key = match inner.get(inner_index) {
                    Some(Token::Literal(s)) => s.to_string(),
                    Some(t) => {
                        return Err(SchemaParseError::UnexpectedToken(t.clone()));
                    },
                    None => { break; },
                };

                inner_index += 1;

                match inner.get(inner_index) {
                    Some(Token::Punct(b':')) => {},
                    Some(t) => {
                        return Err(SchemaParseError::UnexpectedToken(t.clone()));
                    },
                    None => {
                        return Err(SchemaParseError::UnexpectedEof);
                    },
                }

                inner_index += 1;
                let inner_type = token_to_schema(&inner, &mut inner_index)?;
                result.push((key, inner_type));

                match inner.get(inner_index) {
                    Some(Token::Punct(b',')) => {
                        inner_index += 1;
                    },
                    Some(t) => {
                        return Err(SchemaParseError::UnexpectedToken(t.clone()));
                    },
                    None => { break; },
                }
            }

            Schema {
                r#type: SchemaType::Object(result),
                constraint: None,
            }
        },
        Some(Token::Group {
            kind: GroupKind::Bracket,
            tokens: inner,
        }) => {
            let mut inner_index = 0;
            let inner_type = if inner.is_empty() {
                None
            } else {
                let res = token_to_schema(&inner, &mut inner_index)?;

                if inner_index < inner.len() {
                    return Err(SchemaParseError::UnexpectedToken(inner[inner_index].clone()));
                }

                Some(res)
            };

            Schema::default_array(inner_type)
        },
        Some(t) => {
            return Err(SchemaParseError::UnexpectedToken(t.clone()));
        },
        None => {
            return Err(SchemaParseError::UnexpectedEof);
        },
    };
    *index += 1;

    if let Some(Token::Group { kind: GroupKind::Brace, tokens: inner }) = tokens.get(*index) {
        let constraint = parse_constraint(inner)?;
        r#type.add_constraint(constraint);
        *index += 1;
    }

    Ok(r#type)
}

fn parse_constraint(tokens: &[Token]) -> Result<Constraint, SchemaParseError> {
    let mut index = 0;
    let mut result = Constraint::default();

    loop {
        let key = match tokens.get(index) {
            Some(Token::Literal(s)) => s.to_string(),
            Some(t) => {
                return Err(SchemaParseError::UnexpectedToken(t.clone()));
            },
            None => { break; },
        };
        index += 1;

        match tokens.get(index) {
            Some(Token::Punct(b':')) => {},
            Some(t) => {
                return Err(SchemaParseError::UnexpectedToken(t.clone()));
            },
            None => {
                return Err(SchemaParseError::UnexpectedEof);
            },
        }

        index += 1;

        match key.as_str() {
            k @ ("min" | "max" | "len_min" | "len_max") => match tokens.get(index) {
                Some(n @ (Token::Integer(_) | Token::Float(_))) => if k == "min" || k == "len_min" {
                    if result.min.is_some() {
                        return Err(SchemaParseError::InvalidConstraint(format!("A constraint `{key}` appears more than once.")));
                    }

                    result.min = Some(n.to_string());
                } else {
                    if result.max.is_some() {
                        return Err(SchemaParseError::InvalidConstraint(format!("A constraint `{key}` appears more than once.")));
                    }

                    result.max = Some(n.to_string());
                },
                Some(t) => {
                    return Err(SchemaParseError::UnexpectedToken(t.clone()));
                },
                None => {
                    return Err(SchemaParseError::UnexpectedEof);
                },
            },
            _ => {
                return Err(SchemaParseError::InvalidConstraint(format!("`{key}` is not a valid constraint")));
            },
        }

        index += 1;

        match tokens.get(index) {
            Some(Token::Punct(b',')) => {},
            Some(t) => {
                return Err(SchemaParseError::UnexpectedToken(t.clone()));
            },
            None => {
                return Ok(result);
            },
        }

        index += 1;
    }

    Ok(result)
}

fn matching_delim(c: u8) -> u8 {
    match c {
        b'{' => b'}',
        b'(' => b')',
        b'[' => b']',
        b'}' => b'{',
        b')' => b'(',
        b']' => b'[',
        _ => unreachable!(),
    }
}
