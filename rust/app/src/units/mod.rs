//! Written by `rust-anything app lift`: every command is answered in Rust.

use std::process::ExitCode;

use clap::ArgMatches;

mod centiles;

pub fn dispatch(command: &str, matches: &ArgMatches) -> Option<ExitCode> {
    match command {
        "weight" | "height" | "head" => Some(centiles::run(command, matches)),
        _ => None,
    }
}
