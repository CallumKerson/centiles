"""Command line interface for centiles."""

import sys
from typing import Annotated, Literal

from cyclopts import App, Parameter

from centiles.growth import Measurement, Sex, age_for, centile_for, format_age, matching_age, parse_age, value_for

app = App(
    name="centiles",
    help="A CLI for everything to do with baby measurement percentiles.",
)

type SexChoice = Literal["male", "female", "m", "f"]
type SexOption = Annotated[SexChoice, Parameter(env_var="CENTILES_SEX")]

_LABELS: dict[Measurement, str] = {"weight": "Weight", "height": "Height", "head": "Head circumference"}
_UNITS: dict[Measurement, str] = {"weight": "kg", "height": "cm", "head": "cm"}
_SEXES: dict[SexChoice, Sex] = {"male": "male", "female": "female", "m": "male", "f": "female"}


def _ordinal(number: float) -> str:
    rounded = round(number, 1)
    if rounded != int(rounded):
        return f"{rounded:g}th"
    whole = int(rounded)
    suffix = "th" if 11 <= whole % 100 <= 13 else {1: "st", 2: "nd", 3: "rd"}.get(whole % 10, "th")  # noqa: PLR2004
    return f"{whole}{suffix}"


def _solve(
    measurement: Measurement,
    sex: SexOption,
    age: str | None,
    value: float | None,
    centile: float | None,
    compare: float | None,
) -> str:
    if compare is not None:
        if age is None or centile is None or value is not None:
            msg = "Give --compare with --age and --centile."
            raise ValueError(msg)
        found_value, years = matching_age(measurement, _SEXES[sex], parse_age(age), centile, compare)
        return (
            f"{_LABELS[measurement]}: {found_value:.2f} {_UNITS[measurement]}, "
            f"reached on the {_ordinal(compare)} centile at {format_age(years)}"
        )
    if sum(option is not None for option in (age, value, centile)) == 2:  # noqa: PLR2004
        if age is not None and value is not None:
            found, sds = centile_for(measurement, _SEXES[sex], parse_age(age), value)
            direction = "below" if sds < 0 else "above"
            return f"Centile: {_ordinal(found)} ({abs(sds):.2f} standard deviations {direction} average)"
        if age is not None and centile is not None:
            found_value = value_for(measurement, _SEXES[sex], parse_age(age), centile)
            return f"{_LABELS[measurement]}: {found_value:.2f} {_UNITS[measurement]}"
        if value is not None and centile is not None:
            years = age_for(measurement, _SEXES[sex], value, centile)
            return f"Age: {format_age(years)}"
    msg = "Give exactly two of --age, --value and --centile."
    raise ValueError(msg)


def _run(
    measurement: Measurement,
    sex: SexOption,
    age: str | None,
    value: float | None,
    centile: float | None,
    compare: float | None,
) -> None:
    try:
        print(_solve(measurement, sex, age, value, centile, compare))
    except ValueError as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)


@app.command
def weight(
    *,
    sex: SexOption,
    age: str | None = None,
    value: float | None = None,
    centile: float | None = None,
    compare: float | None = None,
) -> None:
    """
    Work out weight, age or centile from the other two.

    Parameters
    ----------
    sex
        The baby's sex.
    age
        Age, like 13w2d, 6m or 1y3m.
    value
        Weight in kg.
    centile
        Centile, between 0 and 100.
    compare
        Comparison centile. Give with --age and --centile to find the age at which a child on this centile has the
        same value.

    """
    _run("weight", sex, age, value, centile, compare)


@app.command
def height(
    *,
    sex: SexOption,
    age: str | None = None,
    value: float | None = None,
    centile: float | None = None,
    compare: float | None = None,
) -> None:
    """
    Work out length or height, age or centile from the other two.

    Parameters
    ----------
    sex
        The baby's sex.
    age
        Age, like 13w2d, 6m or 1y3m.
    value
        Length or height in cm.
    centile
        Centile, between 0 and 100.
    compare
        Comparison centile. Give with --age and --centile to find the age at which a child on this centile has the
        same value.

    """
    _run("height", sex, age, value, centile, compare)


@app.command
def head(
    *,
    sex: SexOption,
    age: str | None = None,
    value: float | None = None,
    centile: float | None = None,
    compare: float | None = None,
) -> None:
    """
    Work out head circumference, age or centile from the other two.

    Parameters
    ----------
    sex
        The baby's sex.
    age
        Age, like 13w2d, 6m or 1y3m.
    value
        Head circumference in cm.
    centile
        Centile, between 0 and 100.
    compare
        Comparison centile. Give with --age and --centile to find the age at which a child on this centile has the
        same value.

    """
    _run("head", sex, age, value, centile, compare)
