"""Growth calculations against the UK-WHO reference."""

import re
from statistics import NormalDist
from typing import Literal

from rcpchgrowth import measurement_from_sds, sds_for_measurement

type Measurement = Literal["weight", "height", "head"]
type Sex = Literal["male", "female"]

REFERENCE = "uk-who"
DAYS_PER_YEAR = 365.25
UNIT_DAYS = {"y": DAYS_PER_YEAR, "m": DAYS_PER_YEAR / 12, "w": 7, "d": 1}
TWO_YEARS = 2
AGE_TOLERANCE = 0.1 / DAYS_PER_YEAR

_RCPCH_NAMES: dict[Measurement, str] = {"weight": "weight", "height": "height", "head": "ofc"}
_AGE_PART = re.compile(r"(\d+(?:\.\d+)?)([ymwd])")
_NORMAL = NormalDist()


def parse_age(text: str) -> float:
    """Parse an age such as ``13w2d``, ``6m`` or ``1y3m`` into decimal years."""
    compact = text.replace(" ", "").lower()
    parts = _AGE_PART.findall(compact)
    if not parts or "".join(number + unit for number, unit in parts) != compact:
        msg = f"Can't read age {text!r}; use units y, m, w and d, like 13w2d or 1y3m."
        raise ValueError(msg)
    return sum(float(number) * UNIT_DAYS[unit] for number, unit in parts) / DAYS_PER_YEAR


def format_age(years: float) -> str:
    """
    Format decimal years for reading.

    Under two years this gives weeks and days, followed by months and days under one year or years and months
    after. From two years it gives years and months.
    """
    if years >= TWO_YEARS:
        return _years_and_months(years)
    days = round(years * DAYS_PER_YEAR)
    weeks = f"{days // 7}w {days % 7}d"
    if years >= 1:
        return f"{weeks} ({_years_and_months(years)})"
    months = int(days / UNIT_DAYS["m"])
    return f"{weeks} ({months}m {days - round(months * UNIT_DAYS['m'])}d)"


def _years_and_months(years: float) -> str:
    months = round(years * 12)
    return f"{months // 12}y {months % 12}m"


def max_age(measurement: Measurement, sex: Sex) -> float:
    """Return the oldest age, in years, the reference covers."""
    if measurement == "head":
        return 18 if sex == "male" else 17
    return 20


def _check_age(measurement: Measurement, sex: Sex, age: float) -> None:
    if not 0 <= age <= max_age(measurement, sex):
        msg = f"Age must be between birth and {max_age(measurement, sex)} years for {measurement}."
        raise ValueError(msg)


def _check_centile(centile: float) -> None:
    if not 0 < centile < 100:  # noqa: PLR2004
        msg = "Centile must be between 0 and 100."
        raise ValueError(msg)


def centile_for(measurement: Measurement, sex: Sex, age: float, value: float) -> tuple[float, float]:
    """Return the centile and standard deviation score (SDS) for a measurement value at an age."""
    _check_age(measurement, sex, age)
    sds = sds_for_measurement(
        reference=REFERENCE,
        age=age,
        measurement_method=_RCPCH_NAMES[measurement],
        observation_value=value,
        sex=sex,
    )
    return _NORMAL.cdf(sds) * 100, float(sds)


def value_for(measurement: Measurement, sex: Sex, age: float, centile: float) -> float:
    """Return the measurement value on a centile at an age."""
    _check_age(measurement, sex, age)
    _check_centile(centile)
    value = measurement_from_sds(
        reference=REFERENCE,
        requested_sds=_NORMAL.inv_cdf(centile / 100),
        measurement_method=_RCPCH_NAMES[measurement],
        sex=sex,
        age=age,
    )
    if value is None:
        msg = f"No {measurement} reference data at that age."
        raise ValueError(msg)
    return float(value)


def age_for(measurement: Measurement, sex: Sex, value: float, centile: float) -> float:
    """Return the age at which a measurement on a centile reaches a value."""
    low, high = 0.0, float(max_age(measurement, sex))
    if value < value_for(measurement, sex, low, centile):
        msg = f"That {measurement} is below the centile's value at birth."
        raise ValueError(msg)
    if value > value_for(measurement, sex, high, centile):
        msg = f"That {measurement} is beyond the centile's value at {high:g} years, the end of the reference."
        raise ValueError(msg)
    while high - low > AGE_TOLERANCE:
        middle = (low + high) / 2
        if value_for(measurement, sex, middle, centile) < value:
            low = middle
        else:
            high = middle
    return (low + high) / 2


def matching_age(measurement: Measurement, sex: Sex, age: float, centile: float, compare: float) -> tuple[float, float]:
    """Return the value on a centile at an age, and the age at which the comparison centile reaches that value."""
    value = value_for(measurement, sex, age, centile)
    _check_centile(compare)
    return value, age_for(measurement, sex, value, compare)
