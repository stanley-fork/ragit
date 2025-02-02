use crate::{ArgCount, ArgType, Span};

pub struct Error {
    pub span: Span,
    pub kind: ErrorKind,
}

pub enum ErrorKind {
    /// see <https://doc.rust-lang.org/stable/std/num/struct.ParseIntError.html>
    ParseIntError(std::num::ParseIntError),

    /// (prev_flag, curr_flag)
    SameFlagMultipleTimes(String, String),

    /// of an arg_flag
    MissingArgument(String, ArgType),

    WrongArgCount {
        expected: ArgCount,
        got: usize,
    },
    MissingFlag(String),

    // TODO: suggest similar names
    UnknownFlag(String),
}

impl ErrorKind {
    pub fn render(&self) -> String {
        match self {
            ErrorKind::ParseIntError(_) => String::from("Cannot parse int."),
            ErrorKind::SameFlagMultipleTimes(prev, next) => if prev == next {
                format!("Flag `{next}` cannot be used multiple times.")
            } else {
                format!("Flag `{prev}` and `{next}` cannot be used together.")
            },
            ErrorKind::MissingArgument(arg, arg_type) => format!(
                "A {} value is required for flag `{arg}`, but is missing.",
                format!("{arg_type:?}").to_ascii_lowercase(),
            ),
            ErrorKind::WrongArgCount { expected, got } => format!(
                "Expected {} arguments, got {got} arguments",
                match expected {
                    ArgCount::Exact(n) => format!("exactly {n}"),
                    ArgCount::Geq(n) => format!("at least {n}"),
                    ArgCount::Leq(n) => format!("at most {n}"),
                    ArgCount::None => String::from("no"),
                    ArgCount::Any => unreachable!(),
                },
            ),
            ErrorKind::MissingFlag(flag) => format!("Flag `{flag}` is missing."),
            ErrorKind::UnknownFlag(flag) => format!("Unknown flag: `{flag}`."),
        }
    }
}
