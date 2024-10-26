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

    pub fn parse(&self, raw_args: &[String]) -> Result<ParsedArgs, Error> {
        todo!()
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
