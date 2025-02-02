use std::collections::HashMap;

mod error;
mod span;

pub use error::{Error, ErrorKind};
pub use span::Span;

pub struct ArgParser {
    arg_count: ArgCount,
    arg_type: ArgType,
    flags: Vec<Flag>,

    // `--N=20`, `--prefix=rust`
    arg_flags: HashMap<String, (Option<String> /* default value */, ArgType)>,
}

impl ArgParser {
    pub fn new() -> Self {
        ArgParser {
            arg_count: ArgCount::None,
            arg_type: ArgType::String,
            flags: vec![],
            arg_flags: HashMap::new(),
        }
    }

    pub fn args(&mut self, arg_type: ArgType, arg_count: ArgCount) -> &mut Self {
        self.arg_type = arg_type;
        self.arg_count = arg_count;
        self
    }

    pub fn flag(&mut self, flags: &[&str]) -> &mut Self {
        self.flags.push(Flag {
            values: flags.iter().map(|flag| flag.to_string()).collect(),
            optional: false,
            default: None,
        });
        self
    }

    pub fn optional_flag(&mut self, flags: &[&str]) -> &mut Self {
        self.flags.push(Flag {
            values: flags.iter().map(|flag| flag.to_string()).collect(),
            optional: true,
            default: None,
        });
        self
    }

    pub fn arg_flag(&mut self, flag: &str, arg_type: ArgType) -> &mut Self {
        self.arg_flags.insert(flag.to_string(), (None, arg_type));
        self
    }

    pub fn optional_arg_flag(&mut self, flag: &str, default: &str, arg_type: ArgType) -> &mut Self {
        self.arg_flags.insert(flag.to_string(), (Some(default.to_string()), arg_type));
        self
    }

    // the first flag is the default value
    pub fn flag_with_default(&mut self, flags: &[&str]) -> &mut Self {
        self.flags.push(Flag {
            values: flags.iter().map(|flag| flag.to_string()).collect(),
            optional: true,
            default: Some(0),
        });
        self
    }

    pub fn parse(&self, raw_args: &[String]) -> Result<ParsedArgs, Error> {
        self.parse_worker(raw_args).map_err(
            |mut e| {
                e.span = e.span.render(raw_args);
                e
            }
        )
    }

    pub fn parse_worker(&self, raw_args: &[String]) -> Result<ParsedArgs, Error> {
        let mut args = vec![];
        let mut flags = vec![None; self.flags.len()];
        let mut arg_flags = HashMap::new();
        let mut expecting_flag_arg: Option<(String, ArgType)> = None;

        if raw_args.get(0).map(|arg| arg.as_str()) == Some("--help") {
            return Ok(ParsedArgs {
                args,
                flags: vec![],
                arg_flags,
                show_help: true,
            });
        }

        'raw_arg_loop: for (arg_index, raw_arg) in raw_args.iter().enumerate() {
            if let Some((flag, arg_type)) = expecting_flag_arg {
                expecting_flag_arg = None;
                arg_type.parse(raw_arg, Span::Exact(arg_index))?;

                if let Some(_) = arg_flags.insert(flag.clone(), raw_arg.to_string()) {
                    return Err(Error {
                        span: Span::Exact(arg_index),
                        kind: ErrorKind::SameFlagMultipleTimes(
                            flag.clone(),
                            flag.clone(),
                        ),
                    });
                }

                continue;
            }

            if raw_arg.starts_with("--") {
                for (flag_index, flag) in self.flags.iter().enumerate() {
                    if flag.values.contains(raw_arg) {
                        if flags[flag_index].is_none() {
                            flags[flag_index] = Some(raw_arg.to_string());
                            continue 'raw_arg_loop;
                        }

                        else {
                            return Err(Error {
                                span: Span::Exact(arg_index),
                                kind: ErrorKind::SameFlagMultipleTimes(
                                    flags[flag_index].as_ref().unwrap().to_string(),
                                    raw_arg.to_string(),
                                ),
                            });
                        }
                    }
                }

                if let Some((_, arg_type)) = self.arg_flags.get(raw_arg) {
                    expecting_flag_arg = Some((raw_arg.to_string(), *arg_type));
                    continue;
                }

                if raw_arg.contains("=") {
                    let splitted = raw_arg.splitn(2, '=').collect::<Vec<_>>();
                    let flag = splitted[0];
                    let flag_arg = splitted[1];

                    if let Some((_, arg_type)) = self.arg_flags.get(flag) {
                        arg_type.parse(flag_arg, Span::Exact(arg_index))?;

                        if let Some(_) = arg_flags.insert(flag.to_string(), flag_arg.to_string()) {
                            return Err(Error {
                                span: Span::Exact(arg_index),
                                kind: ErrorKind::SameFlagMultipleTimes(
                                    flag.to_string(),
                                    flag.to_string(),
                                ),
                            });
                        }

                        continue;
                    }

                    else {
                        return Err(Error {
                            span: Span::Exact(arg_index),
                            kind: ErrorKind::UnknownFlag(flag.to_string()),
                        });
                    }
                }

                return Err(Error {
                    span: Span::Exact(arg_index),
                    kind: ErrorKind::UnknownFlag(raw_arg.to_string()),
                });
            }

            else {
                args.push(self.arg_type.parse(raw_arg, Span::Exact(arg_index))?);
            }
        }

        if let Some((arg, arg_type)) = expecting_flag_arg {
            return Err(Error {
                span: Span::End,
                kind: ErrorKind::MissingArgument(arg.to_string(), arg_type),
            });
        }

        for i in 0..flags.len() {
            if flags[i].is_none() {
                if let Some(j) = self.flags[i].default {
                    flags[i] = Some(self.flags[i].values[j].clone());
                }

                else if !self.flags[i].optional {
                    return Err(Error {
                        span: Span::End,
                        kind: ErrorKind::MissingFlag(self.flags[i].values.join(" | ")),
                    });
                }
            }
        }

        loop {
            let span = match self.arg_count {
                ArgCount::Geq(n) if args.len() < n => { Span::End },
                ArgCount::Leq(n) if args.len() > n => { Span::NthArg(n + 1) },
                ArgCount::Exact(n) if args.len() != n => { Span::NthArg(n + 1) },
                ArgCount::None if args.len() > 0 => { Span::FirstArg },
                _ => { break; },
            };

            return Err(Error {
                span,
                kind: ErrorKind::WrongArgCount {
                    expected: self.arg_count,
                    got: args.len(),
                },
            });
        }

        for (flag, (default, _)) in self.arg_flags.iter() {
            if arg_flags.contains_key(flag) {
                continue;
            }

            else if let Some(default) = default {
                arg_flags.insert(flag.to_string(), default.to_string());
            }

            else {
                return Err(Error {
                    span: Span::End,
                    kind: ErrorKind::MissingFlag(flag.to_string()),
                });
            }
        }

        Ok(ParsedArgs {
            args,
            flags,
            arg_flags,
            show_help: false,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ArgCount {
    Geq(usize),
    Leq(usize),
    Exact(usize),
    Any,
    None,
}

#[derive(Clone, Copy, Debug)]
pub enum ArgType {
    String,
    Path,
    Command,
    Query,  // uid or path
    Integer,
}

impl ArgType {
    pub fn parse(&self, arg: &str, span: Span) -> Result<String, Error> {
        match self {
            ArgType::Integer => match arg.parse::<i64>() {
                Ok(_) => Ok(arg.to_string()),
                Err(e) => Err(Error {
                    span,
                    kind: ErrorKind::ParseIntError(e),
                }),
            },
            ArgType::String
            | ArgType::Path
            | ArgType::Command  // TODO: validator for ArgType::Command
            | ArgType::Query => Ok(arg.to_string()),
        }
    }
}

pub struct Flag {
    values: Vec<String>,
    optional: bool,
    default: Option<usize>,
}

pub struct ParsedArgs {
    args: Vec<String>,
    flags: Vec<Option<String>>,
    pub arg_flags: HashMap<String, String>,
    show_help: bool,  // TODO: options for help messages
}

impl ParsedArgs {
    pub fn get_args(&self) -> Vec<String> {
        self.args.clone()
    }

    pub fn get_args_exact(&self, count: usize) -> Result<Vec<String>, Error> {
        if self.args.len() == count {
            Ok(self.args.clone())
        }

        else {
            Err(Error {
                span: Span::FirstArg,
                kind: ErrorKind::WrongArgCount {
                    expected: ArgCount::Exact(count),
                    got: self.args.len(),
                },
            })
        }
    }

    // if there's an index error, it panics instead of returning None
    // if it returns None, that means Nth flag is optional and its value is None
    pub fn get_flag(&self, index: usize) -> Option<String> {
        self.flags[index].clone()
    }

    pub fn show_help(&self) -> bool {
        self.show_help
    }
}

pub fn underline_span(prefix: &str, args: &str, start: usize, end: usize) -> String {
    format!(
        "{prefix}{args}\n{}{}{}{}",
        " ".repeat(prefix.len()),
        " ".repeat(start),
        "^".repeat(end - start),
        " ".repeat(args.len() - end),
    )
}
