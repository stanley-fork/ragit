use super::SchemaType;
use serde_json::Value;

// It's NOT a json parser!! Its job is to tell a json parser which part of the string to parse.
// It extracts
//   1. Valid json values inside curly braces or square brackets.
//       - If there are multiple curly braces or square brackets, it extracts all of them.
//       - If there's an invalid json literal inside a curly brace or a square bracket, it just ignores the literal.
//       - If there's an unmatched curly brace or square bracket, it warns the LLM.
//   2. Numeric literals.
//       - If there are multiple numeric literals, it extracts all of them.
//       - If the literal is inside another json value, it ignores the literal.
//   3. If there are multiple literals that are evaluated to the same value, it keeps only one of them.
//       - This happens frequently. Most LLMs think and answer (with or without an explicit CoT). In such cases, the json literal
//         appears once in the think tokens and once in the answer tokens.
pub fn extract_jsonish_literal(s: &'_ str) -> JsonishLiteral<'_> {
    let mut state = NaturalLanguageParseState::Init;
    let mut json_stack = vec![];
    let mut start_index = 0;
    let mut result = JsonishLiteral {
        s,
        integers: vec![],
        floats: vec![],
        braces: vec![],
        brackets: vec![],
        likely_to_be_broken_json: false,
    };

    for (index, c) in s.bytes().enumerate() {
        match &mut state {
            NaturalLanguageParseState::Init => match c {
                b'0'..=b'9' | b'-' => {
                    state = NaturalLanguageParseState::Integer;
                    start_index = index;
                },
                b'{' | b'[' => {
                    state = NaturalLanguageParseState::Json(JsonParseState::Init);
                    json_stack = vec![JsonGroup::from(c)];
                    start_index = index;
                },
                b'}' | b']' => {
                    result.likely_to_be_broken_json = true;
                },
                _ => {},
            },
            NaturalLanguageParseState::Integer => match c {
                b'0'..=b'9' => {},
                b'.' => {
                    state = NaturalLanguageParseState::Float;
                },
                _ => {
                    state = NaturalLanguageParseState::Init;
                    result.integers.push((start_index, index));
                    result.floats.push((start_index, index));
                },
            },
            NaturalLanguageParseState::Float => match c {
                b'0'..=b'9' => {},
                _ => {
                    state = NaturalLanguageParseState::Init;
                    result.floats.push((start_index, index));
                },
            },
            // It doesn't have to be a strict json parser. `serde_json` will do that.
            NaturalLanguageParseState::Json(json_state) => match json_state {
                JsonParseState::Init => match c {
                    b'{' | b'[' => {
                        json_stack.push(JsonGroup::from(c));
                    },
                    b'}' | b']' => match json_stack.pop() {
                        Some(jsg) => if jsg == JsonGroup::from(c) {
                            if json_stack.is_empty() {
                                state = NaturalLanguageParseState::Init;

                                if c == b'}' {
                                    result.braces.push((start_index, index + 1));
                                }

                                else {
                                    result.brackets.push((start_index, index + 1));
                                }
                            }
                        } else {
                            // There's no point in parsing this json literal any further
                            state = NaturalLanguageParseState::Init;
                            result.likely_to_be_broken_json = true;
                            break;
                        },
                        None => {
                            // There's no point in parsing this json literal any further
                            state = NaturalLanguageParseState::Init;
                            result.likely_to_be_broken_json = true;
                            break;
                        },
                    },
                    b'"' => {
                        *json_state = JsonParseState::String { escape: false };
                    },
                    _ => {},
                },
                JsonParseState::String { escape } => match (c, &escape) {
                    (b'"', false) => {
                        *json_state = JsonParseState::Init;
                    },
                    (b'\\', false) => {
                        *escape = true;
                    },
                    (_, false) => {},
                    (_, true) => {
                        *escape = false;
                    },
                },
            },
        }
    }

    match state {
        NaturalLanguageParseState::Init => {},
        NaturalLanguageParseState::Integer => {
            result.integers.push((start_index, s.len()));
            result.floats.push((start_index, s.len()));
        },
        NaturalLanguageParseState::Float => {
            result.floats.push((start_index, s.len()));
        },
        NaturalLanguageParseState::Json(_) => {
            result.likely_to_be_broken_json = true;
        },
    }

    result
}

pub struct JsonishLiteral<'a> {
    s: &'a str,
    integers: Vec<(usize, usize)>,
    floats: Vec<(usize, usize)>,
    braces: Vec<(usize, usize)>,
    brackets: Vec<(usize, usize)>,
    pub likely_to_be_broken_json: bool,
}

impl<'a, 'b> JsonishLiteral<'a> {
    pub fn get_matches(&'a mut self, r#type: &'b SchemaType) -> JsonMatch<'a> {
        match r#type {
            SchemaType::Integer => match self.integers.len() {
                0 => match self.floats.len() {
                    // "7." is pushed to `self.floats` but can be treated like an integer.
                    // "7.5" is pushed to `self.floats` and cannot be treated like an integer.
                    // For now, it doesn't treat "7.0" like an integer. It's kinda TODO.
                    1 if self.s.get((self.floats[0].1 - 1)..self.floats[0].1).unwrap() == "." => {
                        JsonMatch::Match(self.s.get(self.floats[0].0..(self.floats[0].1 - 1)).unwrap())
                    },
                    1 => JsonMatch::ExpectedIntegerGotFloat(self.s.get(self.floats[0].0..self.floats[0].1).unwrap()),
                    _ => JsonMatch::NoMatch,
                },
                1 => JsonMatch::Match(self.s.get(self.integers[0].0..self.integers[0].1).unwrap()),
                _ => {
                    let mut parsed_integers = vec![];
                    let mut selected_str = &self.s[..];

                    for (start, end) in self.integers.iter() {
                        let s = &self.s.get(*start..*end).unwrap();
                        let Ok(n) = s.parse::<i128>() else { continue; };

                        // If the LLM outputs the same literal multiple times, that's fine.
                        match parsed_integers.last() {
                            None => { parsed_integers.push(n); selected_str = s; },
                            Some(l) if *l != n => { return JsonMatch::MultipleMatches; },
                            Some(_) => {},
                        }
                    }

                    if parsed_integers.is_empty() {
                        JsonMatch::NoMatch
                    }

                    else {
                        JsonMatch::Match(selected_str)
                    }
                },
            },
            SchemaType::Float => match self.floats.len() {
                0 => JsonMatch::NoMatch,
                // It does 2 things.
                // 1. If a float literal ends with ".", it removes the point.
                //    - "7." is a valid f64 in Rust, but not a valid json value.
                //    - This happens a lot. For example, if the LLM says "the answer is 7.", then the parser would
                //      match "7.".
                // 2. If the LLM outputs the same literal multiple times, it deduplicates the literals.
                _ => {
                    let mut parsed_floats = vec![];
                    let mut selected_str = &self.s[..];

                    for (start, end) in self.floats.iter() {
                        let (start, mut end) = (*start, *end);
                        let s = &self.s.get(start..end).unwrap();
                        let Ok(n) = s.parse::<f64>() else { continue; };

                        if s.ends_with(".") {
                            end -= 1;
                        }

                        // dedup
                        match parsed_floats.last() {
                            None => {
                                parsed_floats.push(n);
                                selected_str = &self.s.get(start..end).unwrap();
                            },
                            Some(l) if *l != n => { return JsonMatch::MultipleMatches; },
                            Some(_) => {},
                        }
                    }

                    if parsed_floats.is_empty() {
                        JsonMatch::NoMatch
                    }

                    else {
                        JsonMatch::Match(selected_str)
                    }
                },
            },
            ty @ (SchemaType::Array(_) | SchemaType::Object(_)) => {
                let l = if let SchemaType::Array(_) = ty { &self.brackets } else { &self.braces };

                match l.len() {
                    0 => JsonMatch::NoMatch,
                    1 => JsonMatch::Match(self.s.get(l[0].0..l[0].1).unwrap()),
                    _ => {
                        let mut parsed_jsons = vec![];
                        let mut selected_str = &self.s[..];

                        for (start, end) in l.iter() {
                            let s = &self.s.get(*start..*end).unwrap();
                            let Ok(n) = serde_json::from_str::<Value>(s) else { self.likely_to_be_broken_json = true; continue; };

                            // If the LLM outputs the same literal multiple times, that's fine.
                            match parsed_jsons.last() {
                                None => { parsed_jsons.push(n); selected_str = s; },
                                Some(l) if *l != n => { return JsonMatch::MultipleMatches; },
                                Some(_) => {},
                            }
                        }

                        if parsed_jsons.is_empty() {
                            JsonMatch::NoMatch
                        }

                        else {
                            JsonMatch::Match(selected_str)
                        }
                    },
                }
            },
            _ => unreachable!(),
        }
    }
}

enum NaturalLanguageParseState {
    Init,
    Integer,
    Float,
    Json(JsonParseState),
}

enum JsonParseState {
    Init,
    String { escape: bool },
}

#[derive(PartialEq)]
enum JsonGroup {
    Brace,
    Bracket,
}

impl From<u8> for JsonGroup {
    fn from(c: u8) -> JsonGroup {
        match c {
            b'{' | b'}' => JsonGroup::Brace,
            b'[' | b']' => JsonGroup::Bracket,
            _ => panic!(),
        }
    }
}

pub enum JsonMatch<'a> {
    NoMatch,
    MultipleMatches,
    Match(&'a str),
    ExpectedIntegerGotFloat(&'a str),
}
