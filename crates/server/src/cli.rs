use crate::error::Error;
use ragit_cli::{ArgParser, ArgType};

pub struct CliArgs {
    // It overrides config.
    pub port_number: Option<u16>,

    // If set, it overrides config.
    pub verbose: bool,

    // If set, it ignores config and disables all the log-related features.
    pub quiet: bool,
    pub config_file: Option<String>,

    // If set and there's no config file, it continues with a default config without any prompt.
    pub force_default_config: bool,
}

pub fn parse_cli_args(args: Vec<String>) -> Result<CliArgs, Error> {
    let parsed_args = ArgParser::new()
        .optional_flag(&["--verbose", "--quiet"])
        .optional_flag(&["--force-default-config"])
        .optional_arg_flag("--port", ArgType::IntegerBetween { min: Some(0), max: Some(65535) })
        .optional_arg_flag("--config", ArgType::Path)
        .short_flag(&["--verbose", "--port"])
        .parse(&args[1..])?;

    Ok(CliArgs {
        port_number: parsed_args.arg_flags.get("--port").map(|n| n.parse::<u16>().unwrap()),
        verbose: parsed_args.get_flag(0).unwrap_or(String::new()) == "--verbose",
        quiet: parsed_args.get_flag(0).unwrap_or(String::new()) == "--quiet",
        config_file: parsed_args.arg_flags.get("--config").map(|f| f.to_string()),
        force_default_config: parsed_args.get_flag(1).is_some(),
    })
}
