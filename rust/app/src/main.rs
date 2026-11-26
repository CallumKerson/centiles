mod cli;
mod units;

use std::process::ExitCode;

fn main() -> ExitCode {
    // Help and usage errors are clap's (ADR-004).
    let matches = cli::command().get_matches();
    if matches.get_flag("version") {
        // The original prints the bare version, where clap would add the name.
        println!("{}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let (command, sub) = matches.subcommand().unwrap_or((cli::ROOT, &matches));
    units::dispatch(command, sub).unwrap_or_else(|| {
        // INFALLIBLE: clap requires a subcommand or --version, and every subcommand is ported.
        let _ = cli::command().print_help();
        ExitCode::from(2)
    })
}
