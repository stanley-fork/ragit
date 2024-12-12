use ragit::Error;
use std::collections::HashMap;

pub struct ArgParser {
    args: Option<(ArgType, ArgCount)>,
    flags: Vec<Flag>,

    // `--N=20`, `--prefix=rust`
    arg_flags: HashMap<String, ArgType>,
}

impl ArgParser {
    pub fn new() -> Self {
        ArgParser {
            args: None,
            flags: vec![],
            arg_flags: HashMap::new(),
        }
    }

    pub fn args(&mut self, arg_type: ArgType, arg_count: ArgCount) -> &mut Self {
        assert!(self.args.is_none());
        self.args = Some((arg_type, arg_count));
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

        'raw_arg_loop: for raw_arg in raw_args.iter() {
            if let Some((flag, arg_type)) = expecting_flag_arg {
                expecting_flag_arg = None;
                arg_type.parse(raw_arg)?;

                if let Some(_) = arg_flags.insert(flag.clone(), raw_arg.to_string()) {
                    return Err(Error::CliError(format!("{flag:?} is given multiple times")));
                }
            }

            if raw_arg.starts_with("--") {
                for (flag_index, flag) in self.flags.iter().enumerate() {
                    if flag.values.contains(raw_arg) {
                        if flags[flag_index].is_none() {
                            flags[flag_index] = Some(raw_arg.to_string());
                            continue 'raw_arg_loop;
                        }

                        else {
                            return Err(Error::CliError(format!("conflicting flags: {:?} vs {raw_arg:?}", flags[flag_index].clone().unwrap())));
                        }
                    }
                }

                if let Some(arg_type) = self.arg_flags.get(raw_arg) {
                    expecting_flag_arg = Some((raw_arg.to_string(), *arg_type));
                    continue;
                }

                if raw_arg.contains("=") {
                    let splitted = raw_arg.splitn(2, '=').collect::<Vec<_>>();
                    let flag = splitted[0];
                    let flag_arg = splitted[1];

                    if let Some(arg_type) = self.arg_flags.get(flag) {
                        arg_type.parse(flag_arg)?;

                        if let Some(_) = arg_flags.insert(flag.to_string(), flag_arg.to_string()) {
                            return Err(Error::CliError(format!("{flag:?} is given multiple times")));
                        }
                    }

                    else {
                        return Err(Error::CliError(format!("unknown flag: {flag:?}")));
                    }
                }

                return Err(Error::CliError(format!("unknown flag: {raw_arg:?}")));
            }

            else {
                if let Some((arg_type, _)) = &self.args {
                    args.push(arg_type.parse(raw_arg)?);
                }

                else {
                    return Err(Error::CliError(format!("unexpected argument: {raw_arg:?}")));
                }
            }
        }

        if let Some((arg, _)) = expecting_flag_arg {
            return Err(Error::CliError(format!("missing argument of {arg:?}")));
        }

        for i in 0..flags.len() {
            if flags[i].is_none() {
                if let Some(j) = self.flags[i].default {
                    flags[i] = Some(self.flags[i].values[j].clone());
                }

                else if !self.flags[i].optional {
                    return Err(Error::CliError(format!("missing flag: {}", self.flags[i].values.join(" | "))));
                }
            }
        }

        if let Some((_, arg_count)) = &self.args {
            match arg_count {
                ArgCount::Geq(n) if args.len() < *n => {
                    return Err(Error::CliError(format!("expected at least {n} arguments, got only {} arguments", args.len())));
                },
                ArgCount::Leq(n) if args.len() > *n => {
                    return Err(Error::CliError(format!("expected at most {n} arguments, got {} arguments", args.len())));
                },
                ArgCount::Exact(n) if args.len() != *n => {
                    return Err(Error::CliError(format!("expected {n} arguments, got {} arguments", args.len())));
                },
                _ => {},
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

pub enum ArgCount {
    Geq(usize),
    Leq(usize),
    Exact(usize),
}

#[derive(Clone, Copy)]
pub enum ArgType {
    String,
    Path,
    Command,
    Query,  // uid or path
}

impl ArgType {
    pub fn parse(&self, arg: &str) -> Result<String, Error> {
        // for now, there's no parsing error
        Ok(arg.to_string())
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
    arg_flags: HashMap<String, String>,
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
            // TODO: nicer error message
            Err(Error::CliError(format!("expected {} arguments, got {} arguments", count, self.args.len())))
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
