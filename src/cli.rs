use ragit::Error;

pub struct ArgParser {}

impl ArgParser {
    pub fn new() -> Self {
        ArgParser {}
    }

    pub fn args(&mut self, arg_type: ArgType, arg_count: ArgCount) -> &mut Self {
        todo!()
    }

    pub fn flag(&mut self, flags: &[&str]) -> &mut Self {
        todo!()
    }

    pub fn optional_flag(&mut self, flags: &[&str]) -> &mut Self {
        todo!()
    }

    pub fn flag_with_default(&mut self, flags: &[&str]) -> &mut Self {
        todo!()
    }

    pub fn parse(&self, args: &[String]) -> Result<ParsedArgs, Error> {
        todo!()
    }
}

pub struct ParsedArgs {}

impl ParsedArgs {
    pub fn get_args(&self) -> Vec<String> {
        todo!()
    }

    pub fn get_args_exact(&self, count: usize) -> Result<Vec<String>, Error> {
        todo!()
    }

    // if there's an index error, it panics instead of returning None
    // if it returns None, that means Nth flag is optional and its value is None
    pub fn get_flag(&self, index: usize) -> Option<String> {
        todo!()
    }

    pub fn show_help(&self) -> bool {
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
