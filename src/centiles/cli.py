"""Command line interface for centiles."""

from cyclopts import App

app = App(
    name="centiles",
    help="A CLI for everything to do with baby measurement percentiles.",
)


@app.command
def hello(name: str = "World") -> None:
    """
    Say hello.

    Parameters
    ----------
    name
        Who to greet.

    """
    print(f"Hello, {name}!")
