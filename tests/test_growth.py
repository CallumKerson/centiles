import pytest

from centiles.growth import Measurement, Sex, age_for, format_age, matching_age, parse_age, value_for


@pytest.mark.parametrize(
    ("text", "days"),
    [
        ("13w2d", 93),
        ("45d", 45),
        ("1y", 365.25),
        ("6m", 182.625),
        ("1y 3m", 456.5625),
    ],
)
def test_parse_age(text: str, days: float) -> None:
    assert parse_age(text) == pytest.approx(days / 365.25)


@pytest.mark.parametrize("text", ["", "3", "3x", "w3", "1y-3m"])
def test_parse_age_rejects_bad_input(text: str) -> None:
    with pytest.raises(ValueError):
        parse_age(text)


@pytest.mark.parametrize(
    ("years", "text"),
    [
        (93 / 365.25, "13w 2d (3m 2d)"),
        (0, "0w 0d (0m 0d)"),
        (176 / 365.25, "25w 1d (5m 24d)"),
        (1.25, "65w 2d (1y 3m)"),
        (2.5, "2y 6m"),
    ],
)
def test_format_age(years: float, text: str) -> None:
    assert format_age(years) == text


@pytest.mark.parametrize(
    ("measurement", "sex", "age"),
    [
        ("weight", "male", 0.5),
        ("height", "female", 3.0),
        ("head", "male", 0.25),
    ],
)
def test_age_round_trip(measurement: Measurement, sex: Sex, age: float) -> None:
    value = value_for(measurement, sex, age, 75)

    assert age_for(measurement, sex, value, 75) == pytest.approx(age, abs=1 / 365.25)


def test_centile_must_be_in_range() -> None:
    with pytest.raises(ValueError):
        value_for("weight", "male", 1, 100)


def test_age_must_be_covered() -> None:
    with pytest.raises(ValueError):
        value_for("head", "female", 18, 50)


def test_matching_age_on_same_centile() -> None:
    value, age = matching_age("height", "male", 0.25, 75, 75)

    assert value == pytest.approx(value_for("height", "male", 0.25, 75))
    assert age == pytest.approx(0.25, abs=1 / 365.25)


def test_matching_age_on_lower_centile_is_older() -> None:
    _, age = matching_age("height", "male", 0.25, 90, 50)

    assert age > 0.25


def test_compare_centile_must_be_in_range() -> None:
    with pytest.raises(ValueError):
        matching_age("weight", "male", 1, 50, 0)
