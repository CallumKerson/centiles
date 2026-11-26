//! The `centiles` command line. Help, usage errors and bad option values are clap's own
//! (ADR-004); everything after parsing matches the original.

use clap::{Arg, ArgAction, Command};

/// The command `units::dispatch` gets when argv names no subcommand.
pub const ROOT: &str = "centiles";

/// Python's `float()` on an option value: surrounding whitespace is ignored.
fn py_float(text: &str) -> Result<f64, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("unable to convert \"{text}\" into float"))
}

fn measurement(name: &'static str, about: &'static str, value: &'static str) -> Command {
    let number = |id: &'static str, help: &'static str| {
        Arg::new(id)
            .long(id)
            .help(help)
            .value_parser(py_float)
            .allow_negative_numbers(true)
            .action(ArgAction::Set)
    };
    Command::new(name)
        .about(about)
        .arg(
            Arg::new("sex")
                .long("sex")
                .help("The baby's sex.")
                .env("CENTILES_SEX")
                .required(true)
                .value_parser(["male", "female", "m", "f"]),
        )
        .arg(
            Arg::new("age")
                .long("age")
                .help("Age, like 13w2d, 6m or 1y3m.")
                .allow_hyphen_values(true),
        )
        .arg(number("value", value))
        .arg(number("centile", "Centile, between 0 and 100."))
        .arg(number(
            "compare",
            "Comparison centile. Give with --age and --centile to find the age at which a child on this \
             centile has the same value.",
        ))
}

pub fn command() -> Command {
    Command::new(ROOT)
        .about("A CLI for everything to do with baby measurement percentiles.")
        .arg_required_else_help(true)
        .arg(
            Arg::new("version")
                .long("version")
                .help("Display application version.")
                .action(ArgAction::SetTrue)
                .exclusive(true),
        )
        .subcommand(measurement(
            "weight",
            "Work out weight, age or centile from the other two.",
            "Weight in kg.",
        ))
        .subcommand(measurement(
            "height",
            "Work out length or height, age or centile from the other two.",
            "Length or height in cm.",
        ))
        .subcommand(measurement(
            "head",
            "Work out head circumference, age or centile from the other two.",
            "Head circumference in cm.",
        ))
}
