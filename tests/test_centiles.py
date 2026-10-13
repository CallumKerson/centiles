import pytest

from centiles.cli import app


def run(capsys: pytest.CaptureFixture[str], *tokens: str) -> tuple[int | str | None, str, str]:
    with pytest.raises(SystemExit) as exit_info:
        app(list(tokens))
    captured = capsys.readouterr()
    return exit_info.value.code, captured.out, captured.err


def test_centile_from_age_and_value(capsys: pytest.CaptureFixture[str]) -> None:
    code, out, _ = run(capsys, "weight", "--sex", "f", "--age", "13w2d", "--value", "6.1")

    assert code == 0
    assert out == "Centile: 61.4th (0.29 standard deviations above average)\n"


def test_value_from_age_and_centile(capsys: pytest.CaptureFixture[str]) -> None:
    code, out, _ = run(capsys, "head", "--sex", "female", "--age", "6m", "--centile", "50")

    assert code == 0
    assert out == "Head circumference: 42.20 cm\n"


def test_age_from_value_and_centile(capsys: pytest.CaptureFixture[str]) -> None:
    code, out, _ = run(capsys, "weight", "--sex", "m", "--value", "9", "--centile", "50")

    assert code == 0
    assert out == "Age: 40w 5d (9m 11d)\n"


def test_height(capsys: pytest.CaptureFixture[str]) -> None:
    code, out, _ = run(capsys, "height", "--sex", "male", "--age", "1y", "--centile", "50")

    assert code == 0
    assert out.startswith("Height: ")
    assert out.endswith(" cm\n")


@pytest.mark.parametrize(
    "tokens",
    [
        ("--age", "3w"),
        ("--age", "3w", "--value", "4", "--centile", "50"),
    ],
)
def test_needs_exactly_two_inputs(capsys: pytest.CaptureFixture[str], tokens: tuple[str, ...]) -> None:
    code, _, err = run(capsys, "weight", "--sex", "m", *tokens)

    assert code == 1
    assert "exactly two" in err


def test_bad_age(capsys: pytest.CaptureFixture[str]) -> None:
    code, _, err = run(capsys, "weight", "--sex", "m", "--age", "3x", "--value", "4")

    assert code == 1
    assert "Can't read age" in err


def test_value_out_of_range(capsys: pytest.CaptureFixture[str]) -> None:
    code, _, err = run(capsys, "weight", "--sex", "m", "--value", "2", "--centile", "50")

    assert code == 1
    assert "below" in err


def test_sex_from_environment(capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("CENTILES_SEX", "female")

    code, out, _ = run(capsys, "weight", "--age", "13w2d", "--value", "6.1")

    assert code == 0
    assert out == "Centile: 61.4th (0.29 standard deviations above average)\n"


def test_sex_option_beats_environment(capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("CENTILES_SEX", "female")

    code, out, _ = run(capsys, "weight", "--sex", "m", "--age", "13w2d", "--value", "6.1")

    assert code == 0
    assert out == "Centile: 33.4th (0.43 standard deviations below average)\n"
