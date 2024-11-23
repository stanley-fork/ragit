use ragit::Error;

pub struct ArgParser {
    args: Option<(ArgType, ArgCount)>,
    flags: Vec<Flag>,
}

impl ArgParser {
    pub fn new() -> Self {
        ArgParser {
            args: None,
            flags: vec![],
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

    // --flag1 --flag2 args
    // see which group each flag belongs to and parse args
    pub fn parse(&self, raw_args: &[String]) -> Result<ParsedArgs, Error> {
        let mut args = vec![];
        let mut flags = vec![None; self.flags.len()];

        if raw_args.get(0).map(|arg| arg.as_str()) == Some("--help") {
            return Ok(ParsedArgs {
                args: vec![],
                flags: vec![],
                show_help: true,
            });
        }

        let mut is_reading_flag = true;

        'raw_arg_loop: for raw_arg in raw_args.iter() {
            if !raw_arg.starts_with("--") {
                is_reading_flag = false;
            }

            if is_reading_flag {
                for (flag_index, flag) in self.flags.iter().enumerate() {
                    if flag.values.contains(raw_arg) {
                        if flags[flag_index].is_none() {
                            flags[flag_index] = Some(raw_arg.to_string());
                            continue 'raw_arg_loop;
                        }

                        else {
                            return Err(Error::CliError(format!("conflicting flags: {} vs {raw_arg}", flags[flag_index].clone().unwrap())));
                        }
                    }
                }

                return Err(Error::CliError(format!("unknown flag: {raw_arg}")));
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
            show_help: false,
        })
    }
}

pub enum ArgCount {
    Geq(usize),
    Leq(usize),
    Exact(usize),
}

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
