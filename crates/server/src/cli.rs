use crate::error::Error;
use ragit_cli::{ArgParser, ArgType};

#[derive(Clone, Debug)]
pub enum CliCommand {
    Run(RunArgs),
    DropAll(DropArgs),
    TruncateAll(DropArgs),
}

#[derive(Clone, Debug)]
pub struct RunArgs {
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

#[derive(Clone, Copy, Debug)]
pub struct DropArgs {
    pub force: bool,
}

pub fn parse_cli_args(args: Vec<String>) -> Result<CliCommand, Error> {
    match args.get(1).map(|arg| arg.as_str()) {
        Some("run") => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--verbose", "--quiet"])
                .optional_flag(&["--force-default-config"])
                .optional_arg_flag("--port", ArgType::IntegerBetween { min: Some(0), max: Some(65535) })
                .optional_arg_flag("--config", ArgType::Path)
                .short_flag(&["--verbose", "--port"])
                .parse(&args[2..])?;

            Ok(CliCommand::Run(RunArgs {
                port_number: parsed_args.arg_flags.get("--port").map(|n| n.parse::<u16>().unwrap()),
                verbose: parsed_args.get_flag(0).unwrap_or(String::new()) == "--verbose",
                quiet: parsed_args.get_flag(0).unwrap_or(String::new()) == "--quiet",
                config_file: parsed_args.arg_flags.get("--config").map(|f| f.to_string()),
                force_default_config: parsed_args.get_flag(1).is_some(),
            }))
        },
        Some(c @ ("drop-all" | "truncate-all")) => {
            let parsed_args = ArgParser::new()
                .optional_flag(&["--force"])
                .short_flag(&["--force"])
                .parse(&args[2..])?;

            let args = DropArgs {
                force: parsed_args.get_flag(0).is_some(),
            };

            Ok(if c == "drop-all" { CliCommand::DropAll(args) } else { CliCommand::TruncateAll(args) })
        },
        // TODO: CliError
        // TODO: suggest similar commands
        Some(invalid_command) => panic!("invalid command: `{invalid_command}`"),
        // TODO: help message
        None => panic!(),
    }
}
