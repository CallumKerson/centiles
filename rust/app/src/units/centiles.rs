//! Glue from the front door to `centiles-rs`: the port of `src/centiles/cli.py` after parsing,
//! `_solve` and `_run`.

use std::process::ExitCode;

use centiles_rs::port;
use clap::ArgMatches;

fn label(measurement: &str) -> &'static str {
    match measurement {
        "weight" => "Weight",
        "height" => "Height",
        _ => "Head circumference",
    }
}

fn unit(measurement: &str) -> &'static str {
    if measurement == "weight" { "kg" } else { "cm" }
}

/// Python's `f"{x:.2f}"`.
fn fixed2(x: f64) -> String {
    match x {
        x if x.is_nan() => "nan".into(),
        x if x.is_infinite() => (if x > 0.0 { "inf" } else { "-inf" }).into(),
        x => format!("{x:.2}"),
    }
}

/// `_ordinal`: a centile as 61.4th or 3rd.
fn ordinal(number: f64) -> Result<String, String> {
    // Python's round(number, 1), correctly rounded through its decimal form.
    let rounded: f64 = if number.is_finite() {
        format!("{number:.1}").parse().unwrap_or(number)
    } else {
        number
    };
    if rounded.is_nan() {
        return Err("ValueError: cannot convert float NaN to integer".into()); // recorded-literal: CPython's int(nan) error.
    }
    if rounded.is_infinite() {
        return Err("OverflowError: cannot convert float infinity to integer".into()); // recorded-literal: CPython's int(inf) error.
    }
    if rounded != rounded.trunc() {
        return Ok(format!("{rounded}th"));
    }
    let whole = rounded as i64;
    let suffix = match (whole.rem_euclid(100), whole.rem_euclid(10)) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    Ok(format!("{whole}{suffix}"))
}

/// `_solve`: the line to print, or the error as `Kind: message`.
fn solve(
    measurement: &str,
    sex: &str,
    age: Option<&str>,
    value: Option<f64>,
    centile: Option<f64>,
    compare: Option<f64>,
) -> Result<String, String> {
    let (m, s) = (measurement.to_string(), sex.to_string());
    let parse_age = |age: &str| port::parse_age(age.to_string());
    if let Some(compare) = compare {
        let (Some(age), Some(centile), None) = (age, centile, value) else {
            return Err("ValueError: Give --compare with --age and --centile.".into());
        };
        let (found, years) = port::matching_age(m, s, parse_age(age)?, centile, compare)?;
        return Ok(format!(
            "{}: {} {}, reached on the {} centile at {}",
            label(measurement),
            fixed2(found),
            unit(measurement),
            ordinal(compare)?,
            port::format_age(years)?
        ));
    }
    match (age, value, centile) {
        (Some(age), Some(value), None) => {
            let (found, sds) = port::centile_for(m, s, parse_age(age)?, value)?;
            let direction = if sds < 0.0 { "below" } else { "above" };
            Ok(format!(
                "Centile: {} ({} standard deviations {direction} average)",
                ordinal(found)?,
                fixed2(sds.abs())
            ))
        }
        (Some(age), None, Some(centile)) => {
            let found = port::value_for(m, s, parse_age(age)?, centile)?;
            Ok(format!(
                "{}: {} {}",
                label(measurement),
                fixed2(found),
                unit(measurement)
            ))
        }
        (None, Some(value), Some(centile)) => Ok(format!(
            "Age: {}",
            port::format_age(port::age_for(m, s, value, centile)?)?
        )),
        _ => Err("ValueError: Give exactly two of --age, --value and --centile.".into()),
    }
}

/// `_run`: print the answer, or `Error: <message>` with exit code 1. The original printed a
/// traceback for errors other than `ValueError`; the port reports those the same way (ADR-005).
pub fn run(command: &str, matches: &ArgMatches) -> ExitCode {
    let sex = match matches.get_one::<String>("sex").map(String::as_str) {
        Some("male" | "m") => "male",
        _ => "female",
    };
    let result = solve(
        command,
        sex,
        matches.get_one::<String>("age").map(String::as_str),
        matches.get_one::<f64>("value").copied(),
        matches.get_one::<f64>("centile").copied(),
        matches.get_one::<f64>("compare").copied(),
    );
    match result {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let message = error.split_once(": ").map_or(error.as_str(), |(_, m)| m);
            eprintln!("Error: {message}");
            ExitCode::FAILURE
        }
    }
}
