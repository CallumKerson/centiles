import pytest

from centiles.cli import app


def test_hello(capsys: pytest.CaptureFixture[str]) -> None:
    with pytest.raises(SystemExit) as exit_info:
        app(["hello", "--name", "Ada"])

    assert exit_info.value.code == 0
    assert capsys.readouterr().out == "Hello, Ada!\n"
