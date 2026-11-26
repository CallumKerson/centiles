//! The Rust port of `src/centiles/growth.py`, with the parts of rcpchgrowth it uses.
//!
//! rcpchgrowth's `sds_for_measurement` and `measurement_from_sds` are ported for the UK-WHO
//! reference only, over the weight, height and head circumference tables, which are built in
//! from `data/uk_who.json`. Python's float semantics are kept where they reach the output:
//! `round`, `repr`, `sum` and the errors the original raises, including those raised by
//! rcpchgrowth and SciPy at the edges of the reference tables.

#![deny(unsafe_code)]

use std::sync::OnceLock;

use regex::Regex;

use crate::pynum::Num;

const DAYS_PER_YEAR: f64 = 365.25;
const DAYS_PER_MONTH: f64 = DAYS_PER_YEAR / 12.0;
const TWO_YEARS: f64 = 2.0;
const AGE_TOLERANCE: f64 = 0.1 / DAYS_PER_YEAR;
const SQRT2: f64 = std::f64::consts::SQRT_2;
/// Ages, in years, at which UK-WHO switches between its source tables.
const FORTY_TWO_WEEKS: f64 = (42.0 * 7.0 - 40.0 * 7.0) / DAYS_PER_YEAR;
const FOUR_YEARS: f64 = 4.0;

// ---------------------------------------------------------------------------------------------
// Python semantics.

/// Python's `round(x, digits)` on a float: correctly rounded, ties to even.
fn py_round(x: f64, digits: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{x:.digits$}").parse().unwrap_or(x)
}

/// NumPy's `round(x, digits)`: scale, round half to even, scale back.
fn np_round(x: f64, digits: i32) -> f64 {
    let scale = 10f64.powi(digits);
    (x * scale).round_ties_even() / scale
}

/// Python's `round(x)` to an int.
fn py_round_int(x: f64) -> Result<i64, String> {
    if x.is_nan() {
        return Err("ValueError: cannot convert float NaN to integer".into()); // recorded-literal: CPython's float-to-int conversion error.
    }
    if x.is_infinite() {
        return Err("OverflowError: cannot convert float infinity to integer".into()); // recorded-literal: CPython's float-to-int conversion error.
    }
    Ok(x.round_ties_even() as i64)
}

/// Python's `repr` of a float.
fn py_float_repr(x: f64) -> String {
    let text = format!("{x:e}");
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    if !(-4..16).contains(&exponent) {
        let sign = if exponent < 0 { '-' } else { '+' };
        return format!("{mantissa}e{sign}{:02}", exponent.abs());
    }
    let plain = format!("{x}");
    if plain.contains('.') {
        plain
    } else {
        format!("{plain}.0")
    }
}

/// Whether Python's `str.isprintable` holds for one character.
fn py_printable(c: char) -> bool {
    use unicode_general_category::{GeneralCategory as G, get_general_category};
    c == ' '
        || !matches!(
            get_general_category(c),
            G::Control
                | G::Format
                | G::Surrogate
                | G::PrivateUse
                | G::Unassigned
                | G::LineSeparator
                | G::ParagraphSeparator
                | G::SpaceSeparator
        )
}

/// Python's `repr` of a str.
fn py_str_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            ' '..='~' => out.push(c),
            c if (c as u32) > 0x7f && py_printable(c) => out.push(c),
            c if (c as u32) < 0x100 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c if (c as u32) < 0x10000 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push_str(&format!("\\U{:08x}", c as u32)),
        }
    }
    out.push(quote);
    out
}

/// Python's `sum` over floats, which compensates for rounding (Neumaier) since 3.12.
fn py_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let (mut total, mut compensation) = (0.0_f64, 0.0_f64);
    for x in values {
        let t = total + x;
        if total.abs() >= x.abs() {
            compensation += (total - t) + x;
        } else {
            compensation += (x - t) + total;
        }
        total = t;
    }
    if compensation != 0.0 && compensation.is_finite() {
        total += compensation;
    }
    total
}

/// The value of a Unicode decimal digit, as Python's `float` reads it. Decimal digits come in
/// runs of ten from zero, so the value is the distance from the start of its stretch, modulo 10.
fn digit_value(c: char, is_digit: &Regex) -> u32 {
    let mut start = c as u32;
    while let Some(before) = start.checked_sub(1).and_then(char::from_u32) {
        if !is_digit.is_match(before.encode_utf8(&mut [0; 4])) {
            break;
        }
        start -= 1;
    }
    (c as u32 - start) % 10
}

// Python's `math.erfc` is the C library's, which differs from the `libm` crate's in the last
// bit; take the same function so centiles match to the bit on every platform.
#[expect(unsafe_code, reason = "the C library's erfc, as Python calls it")]
// SAFETY: `erfc` from the C maths library takes and returns a double, reads no memory and is
// defined for every input, NaN and infinities included.
unsafe extern "C" {
    safe fn erfc(x: f64) -> f64;
}

/// `statistics.NormalDist().cdf(x)`.
fn normal_cdf(x: f64) -> f64 {
    0.5 * erfc((0.0 - x) / (1.0 * SQRT2))
}

/// Evaluate a polynomial by Horner's rule, coefficients from the highest power, as AS241 does.
fn horner(r: f64, coefficients: &[f64]) -> f64 {
    coefficients[1..]
        .iter()
        .fold(coefficients[0], |acc, &c| acc * r + c)
}

/// `statistics.NormalDist().inv_cdf(p)` for `0 < p < 1`: Wichura's AS241.
#[expect(clippy::excessive_precision, reason = "coefficients as published")]
fn normal_inv_cdf(p: f64) -> f64 {
    let q = p - 0.5;
    if q.abs() <= 0.425 {
        let r = 0.180625 - q * q;
        let num = horner(
            r,
            &[
                2.50908_09287_30122_6727e+3,
                3.34305_75583_58812_8105e+4,
                6.72657_70927_00870_0853e+4,
                4.59219_53931_54987_1457e+4,
                1.37316_93765_50946_1125e+4,
                1.97159_09503_06551_4427e+3,
                1.33141_66789_17843_7745e+2,
                3.38713_28727_96366_6080e+0,
            ],
        ) * q;
        let den = horner(
            r,
            &[
                5.22649_52788_52854_5610e+3,
                2.87290_85735_72194_2674e+4,
                3.93078_95800_09271_0610e+4,
                2.12137_94301_58659_5867e+4,
                5.39419_60214_24751_1077e+3,
                6.87187_00749_20579_0830e+2,
                4.23133_30701_60091_1252e+1,
                1.0,
            ],
        );
        return 0.0 + num / den * 1.0;
    }
    let r = (-(if q <= 0.0 { p } else { 1.0 - p }).ln()).sqrt();
    let (num, den) = if r <= 5.0 {
        let r = r - 1.6;
        (
            horner(
                r,
                &[
                    7.74545_01427_83414_07640e-4,
                    2.27238_44989_26918_45833e-2,
                    2.41780_72517_74506_11770e-1,
                    1.27045_82524_52368_38258e+0,
                    3.64784_83247_63204_60504e+0,
                    5.76949_72214_60691_40550e+0,
                    4.63033_78461_56545_29590e+0,
                    1.42343_71107_49683_57734e+0,
                ],
            ),
            horner(
                r,
                &[
                    1.05075_00716_44416_84324e-9,
                    5.47593_80849_95344_94600e-4,
                    1.51986_66563_61645_71966e-2,
                    1.48103_97642_74800_74590e-1,
                    6.89767_33498_51000_04550e-1,
                    1.67638_48301_83803_84940e+0,
                    2.05319_16266_37758_82187e+0,
                    1.0,
                ],
            ),
        )
    } else {
        let r = r - 5.0;
        (
            horner(
                r,
                &[
                    2.01033_43992_92288_13265e-7,
                    2.71155_55687_43487_57815e-5,
                    1.24266_09473_88078_43860e-3,
                    2.65321_89526_57612_30930e-2,
                    2.96560_57182_85048_91230e-1,
                    1.78482_65399_17291_33580e+0,
                    5.46378_49111_64114_36990e+0,
                    6.65790_46435_01103_77720e+0,
                ],
            ),
            horner(
                r,
                &[
                    2.04426_31033_89939_78564e-15,
                    1.42151_17583_16445_88870e-7,
                    1.84631_83175_10054_68180e-5,
                    7.86869_13114_56132_59100e-4,
                    1.48753_61290_85061_48525e-2,
                    1.36929_88092_27358_05310e-1,
                    5.99832_20655_58879_37690e-1,
                    1.0,
                ],
            ),
        )
    };
    let x = num / den;
    0.0 + (if q < 0.0 { -x } else { x }) * 1.0
}

// ---------------------------------------------------------------------------------------------
// The UK-WHO reference, from rcpchgrowth.

/// One row of a reference table: decimal age, then L, M and S.
type Row = [f64; 4];

/// Rows by measurement (weight, height, head) and sex (male, female).
type Table = [[Vec<Row>; 2]; 3];

/// The four source tables of UK-WHO, youngest first.
fn tables() -> &'static [Table; 4] {
    static TABLES: OnceLock<[Table; 4]> = OnceLock::new();
    TABLES.get_or_init(|| {
        let data: serde_json::Value = serde_json::from_str(include_str!("../data/uk_who.json"))
            // INFALLIBLE: the file is built in, and the tests load it.
            .expect("data/uk_who.json is valid JSON");
        ["uk90_preterm", "who_infants", "who_children", "uk90_child"].map(|name| {
            ["weight", "height", "ofc"].map(|measurement| {
                ["male", "female"].map(|sex| {
                    serde_json::from_value(data[name][measurement][sex].clone())
                        // INFALLIBLE: the file is built in, and the tests load it.
                        .expect("data/uk_who.json has rows of four numbers")
                })
            })
        })
    })
}

#[derive(Clone, Copy)]
enum Measurement {
    Weight,
    Height,
    Head,
}

impl Measurement {
    /// `_RCPCH_NAMES[measurement]`.
    fn parse(name: &str) -> Result<Self, String> {
        match name {
            "weight" => Ok(Self::Weight),
            "height" => Ok(Self::Height),
            "head" => Ok(Self::Head),
            _ => Err(format!("KeyError: {}", py_str_repr(name))),
        }
    }
}

/// `sex` as an index into a table, or rcpchgrowth's `KeyError` for any other name.
fn sex_index(sex: &str) -> Result<usize, String> {
    match sex {
        "male" => Ok(0),
        "female" => Ok(1),
        _ => Err(format!("KeyError: {}", py_str_repr(sex))),
    }
}

/// `uk_who_lms_array_for_measurement_and_sex`, for the ages `_check_age` lets through.
fn reference_rows(age: f64, measurement: Measurement, sex: usize) -> &'static [Row] {
    let table = if age < FORTY_TWO_WEEKS {
        0
    } else if age < TWO_YEARS {
        1
    } else if age < FOUR_YEARS {
        2
    } else {
        3
    };
    &tables()[table][measurement as usize][sex]
}

/// L, M and S at an age. `numpy` marks values SciPy interpolated linearly: they then follow
/// NumPy's arithmetic, which rounds differently and gives `nan` or `inf` where Python raises.
struct Lms {
    l: f64,
    m: f64,
    s: f64,
    numpy: bool,
}

fn nearest_lowest_index(rows: &[Row], age: f64) -> usize {
    let target = py_round(age, 4);
    let mut lowest = 0;
    for (index, row) in rows.iter().enumerate() {
        if py_round(row[0], 4) == target {
            return index;
        }
        if row[0] < age {
            lowest = index;
        }
    }
    lowest
}

/// rcpchgrowth's `cubic_interpolation` through four rows, in the original's order of operations.
fn cubic_interpolation(t: f64, rows: [Row; 4], k: usize) -> f64 {
    let [r0, r1, r2, r3] = rows;
    let (tt0, tt1, tt2, tt3) = (t - r0[0], t - r1[0], t - r2[0], t - r3[0]);
    let (t01, t02, t03) = (r0[0] - r1[0], r0[0] - r2[0], r0[0] - r3[0]);
    let (t12, t13, t23) = (r1[0] - r2[0], r1[0] - r3[0], r2[0] - r3[0]);
    r0[k] * tt1 * tt2 * tt3 / t01 / t02 / t03 - r1[k] * tt0 * tt2 * tt3 / t01 / t12 / t13
        + r2[k] * tt0 * tt1 * tt3 / t02 / t12 / t23
        - r3[k] * tt0 * tt1 * tt2 / t03 / t13 / t23
}

/// `fetch_lms`: the row at a matching age, or cubic interpolation, or SciPy's linear `interp1d`
/// at the ends of a table.
fn fetch_lms(age: f64, rows: &[Row]) -> Result<Lms, String> {
    let index = nearest_lowest_index(rows, age);
    let below = rows[index];
    if py_round(age, 4) == py_round(below[0], 4) {
        return Ok(Lms {
            l: below[1],
            m: below[2],
            s: below[3],
            numpy: false,
        });
    }
    let above = *rows
        .get(index + 1)
        .ok_or("IndexError: list index out of range")?; // recorded-literal: CPython's list index error, raised in rcpchgrowth's fetch_lms.
    if index >= 1 && index + 2 < rows.len() {
        let around = [rows[index - 1], below, above, rows[index + 2]];
        let cubic = |k| cubic_interpolation(age, around, k);
        return Ok(Lms {
            l: cubic(1),
            m: cubic(2),
            s: cubic(3),
            numpy: false,
        });
    }
    if age < below[0] {
        return Err(format!(
            "ValueError: A value ({}) in x_new is below the interpolation range's minimum value ({}).", // recorded-literal: SciPy interp1d's bounds error.
            py_float_repr(age),
            py_float_repr(below[0])
        ));
    }
    let linear =
        |k: usize| (above[k] - below[k]) / (above[0] - below[0]) * (age - below[0]) + below[k];
    Ok(Lms {
        l: linear(1),
        m: linear(2),
        s: linear(3),
        numpy: true,
    })
}

/// rcpchgrowth's LMS at an age, with `_age_for_lms_lookup`'s clamp at exactly two years.
fn lms_at(age: f64, measurement: Measurement, sex: usize) -> Result<Lms, String> {
    let rows = reference_rows(age, measurement, sex);
    let lookup_age = if age == TWO_YEARS {
        age.max(rows[0][0]).min(rows[rows.len() - 1][0])
    } else {
        age
    };
    fetch_lms(lookup_age, rows)
}

/// `sds_for_measurement`. Python raises where its float `**` would give a complex number or
/// divide by zero; NumPy gives `nan` or `inf` instead.
fn sds_for_measurement(
    age: f64,
    measurement: Measurement,
    sex: usize,
    value: f64,
) -> Result<f64, String> {
    let Lms { l, m, s, numpy } = lms_at(age, measurement, sex)?;
    if l == 0.0 {
        return Ok((value / m).ln() / s);
    }
    let base = value / m;
    if !numpy && base == 0.0 && l < 0.0 {
        return Err("ZeroDivisionError: zero to a negative power".into()); // recorded-literal: CPython's float power error, raised in rcpchgrowth's z_score.
    }
    if !numpy && base < 0.0 && l.fract() != 0.0 {
        return Err("TypeError: must be real number, not complex".into()); // recorded-literal: CPython's error when math.erfc gets the complex z_score.
    }
    Ok((base.powf(l) - 1.0) / (l * s))
}

/// `measurement_from_sds`: `None` where the LMS formula has no real answer.
fn measurement_from_sds(
    sds: f64,
    measurement: Measurement,
    sex: usize,
    age: f64,
) -> Result<Option<f64>, String> {
    let Lms { l, m, s, numpy } = lms_at(age, measurement, sex)?;
    let value = if l == 0.0 {
        (s * sds).exp() * m
    } else {
        let first_step = 1.0 + l * s * sds;
        if first_step < 0.0 || (!numpy && first_step == 0.0 && l < 0.0) {
            return Ok(None);
        }
        first_step.powf(1.0 / l) * m
    };
    Ok(Some(if numpy {
        np_round(value, 4)
    } else {
        py_round(value, 4)
    }))
}

// ---------------------------------------------------------------------------------------------
// growth.py.

/// `_AGE_PART`, and a test for one Unicode decimal digit.
fn age_regexes() -> &'static (Regex, Regex) {
    static REGEXES: OnceLock<(Regex, Regex)> = OnceLock::new();
    REGEXES.get_or_init(|| {
        (
            // INFALLIBLE: constant patterns.
            Regex::new(r"(\d+(?:\.\d+)?)([ymwd])").expect("valid pattern"),
            Regex::new(r"^\d$").expect("valid pattern"),
        )
    })
}

/// Port of `parse_age` (`src/centiles/growth.py:23`).
pub fn parse_age(text: String) -> Result<f64, String> {
    let (part, is_digit) = age_regexes();
    let compact = text.replace(' ', "").to_lowercase();
    let parts: Vec<(&str, &str)> = part
        .captures_iter(&compact)
        .map(|c| {
            (
                c.get(1).map_or("", |m| m.as_str()),
                c.get(2).map_or("", |m| m.as_str()),
            )
        })
        .collect();
    if parts.is_empty()
        || parts
            .iter()
            .map(|(n, u)| format!("{n}{u}"))
            .collect::<String>()
            != compact
    {
        return Err(format!(
            "ValueError: Can't read age {}; use units y, m, w and d, like 13w2d or 1y3m.",
            py_str_repr(&text)
        ));
    }
    let days = parts.iter().map(|(number, unit)| {
        let ascii: String = number
            .chars()
            .map(|c| match c {
                '.' => '.',
                c => char::from_digit(digit_value(c, is_digit), 10).unwrap_or('0'),
            })
            .collect();
        let unit_days = match *unit {
            "y" => DAYS_PER_YEAR,
            "m" => DAYS_PER_MONTH,
            "w" => 7.0,
            _ => 1.0,
        };
        ascii.parse::<f64>().unwrap_or(f64::NAN) * unit_days
    });
    Ok(py_sum(days) / DAYS_PER_YEAR)
}

/// Port of `format_age` (`src/centiles/growth.py:33`).
pub fn format_age(years: f64) -> Result<String, String> {
    if years >= TWO_YEARS {
        return years_and_months(years);
    }
    let days = py_round_int(years * DAYS_PER_YEAR)?;
    let weeks = format!("{}w {}d", days.div_euclid(7), days.rem_euclid(7));
    if years >= 1.0 {
        return Ok(format!("{weeks} ({})", years_and_months(years)?));
    }
    let months = (days as f64 / DAYS_PER_MONTH).trunc() as i64;
    let rest = days - py_round_int(months as f64 * DAYS_PER_MONTH)?;
    Ok(format!("{weeks} ({months}m {rest}d)"))
}

fn years_and_months(years: f64) -> Result<String, String> {
    let months = py_round_int(years * 12.0)?;
    Ok(format!(
        "{}y {}m",
        months.div_euclid(12),
        months.rem_euclid(12)
    ))
}

fn oldest(measurement: &str, sex: &str) -> i64 {
    match (measurement, sex) {
        ("head", "male") => 18,
        ("head", _) => 17,
        _ => 20,
    }
}

/// Port of `max_age` (`src/centiles/growth.py:55`).
pub fn max_age(measurement: String, sex: String) -> Result<Num, String> {
    Ok(Num::Int(oldest(&measurement, &sex)))
}

fn check_age(measurement: &str, sex: &str, age: f64) -> Result<(), String> {
    let oldest = oldest(measurement, sex);
    if !(0.0..=oldest as f64).contains(&age) {
        return Err(format!(
            "ValueError: Age must be between birth and {oldest} years for {measurement}."
        ));
    }
    Ok(())
}

fn check_centile(centile: f64) -> Result<(), String> {
    if !(centile > 0.0 && centile < 100.0) {
        return Err("ValueError: Centile must be between 0 and 100.".into());
    }
    Ok(())
}

/// Port of `centile_for` (`src/centiles/growth.py:74`).
pub fn centile_for(
    measurement: String,
    sex: String,
    age: f64,
    value: f64,
) -> Result<(f64, f64), String> {
    check_age(&measurement, &sex, age)?;
    let kind = Measurement::parse(&measurement)?;
    let sds = sds_for_measurement(age, kind, sex_index(&sex)?, value)?;
    Ok((normal_cdf(sds) * 100.0, sds))
}

/// Port of `value_for` (`src/centiles/growth.py:87`).
pub fn value_for(measurement: String, sex: String, age: f64, centile: f64) -> Result<f64, String> {
    check_age(&measurement, &sex, age)?;
    check_centile(centile)?;
    let kind = Measurement::parse(&measurement)?;
    let sds = normal_inv_cdf(centile / 100.0);
    measurement_from_sds(sds, kind, sex_index(&sex)?, age)?
        .ok_or_else(|| format!("ValueError: No {measurement} reference data at that age."))
}

/// Port of `age_for` (`src/centiles/growth.py:104`).
pub fn age_for(measurement: String, sex: String, value: f64, centile: f64) -> Result<f64, String> {
    let oldest = oldest(&measurement, &sex);
    let at = |age: f64| value_for(measurement.clone(), sex.clone(), age, centile);
    let (mut low, mut high) = (0.0, oldest as f64);
    if value < at(low)? {
        return Err(format!(
            "ValueError: That {measurement} is below the centile's value at birth."
        ));
    }
    if value > at(high)? {
        return Err(format!(
            "ValueError: That {measurement} is beyond the centile's value at {oldest} years, the end of the reference."
        ));
    }
    while high - low > AGE_TOLERANCE {
        let middle = (low + high) / 2.0;
        if at(middle)? < value {
            low = middle;
        } else {
            high = middle;
        }
    }
    Ok((low + high) / 2.0)
}

/// Port of `matching_age` (`src/centiles/growth.py:122`).
pub fn matching_age(
    measurement: String,
    sex: String,
    age: f64,
    centile: f64,
    compare: f64,
) -> Result<(f64, f64), String> {
    let value = value_for(measurement.clone(), sex.clone(), age, centile)?;
    check_centile(compare)?;
    Ok((value, age_for(measurement, sex, value, compare)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_load() {
        let t = tables();
        assert_eq!(t[1][0][0].len(), 731);
        assert_eq!(t[3][2][1].last().map(|r| r[0]), Some(17.0));
    }

    #[test]
    fn rounding_matches_python() {
        assert_eq!(py_round(125.73005, 4), 125.7301);
        assert_eq!(np_round(125.73005, 4), 125.73);
        assert_eq!(format!("{:.2}", 0.125), "0.12");
        assert_eq!(py_round_int(2.5), Ok(2));
    }

    #[test]
    fn reprs_match_python() {
        assert_eq!(py_float_repr(2.0), "2.0");
        assert_eq!(py_float_repr(1e-5), "1e-05");
        assert_eq!(py_float_repr(1e16), "1e+16");
        assert_eq!(py_str_repr("1y'"), "\"1y'\"");
        assert_eq!(py_str_repr("1y\u{a0}3m"), "'1y\\xa03m'");
    }

    #[test]
    fn ages_parse_and_format() {
        assert_eq!(parse_age("1y3m".into()), Ok(1.25));
        assert_eq!(parse_age("٢y".into()), Ok(2.0));
        assert_eq!(format_age(0.25).as_deref(), Ok("13w 0d (2m 30d)"));
        assert_eq!(format_age(2.5).as_deref(), Ok("2y 6m"));
    }

    #[test]
    fn normal_distribution_round_trips() {
        assert!((normal_cdf(normal_inv_cdf(0.9)) - 0.9).abs() < 1e-12);
        assert_eq!(normal_inv_cdf(0.5), 0.0);
    }
}
