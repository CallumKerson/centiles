# centiles

A CLI for everything to do with baby measurement percentiles.

## Usage

There's a command for each measurement: `weight` (kg), `height` (length or
height, cm) and `head` (head circumference, cm). Give the baby's sex and any two
of age, value and centile, and it works out the third against the UK-WHO
reference:

```sh
centiles weight --sex f --age 13w2d --value 6.1   # Centile: 61.4th (0.29 standard deviations above average)
centiles weight --sex f --age 1y --centile 50     # Weight: 8.95 kg
centiles weight --sex m --value 9 --centile 50    # Age: 40w 5d (9m 11d)
centiles head --sex female --age 6m --centile 50  # Head circumference: 42.20 cm
```

- `--sex` is `male`, `female`, `m` or `f`. Set `CENTILES_SEX` to avoid passing
  it every time; `--sex` still wins when given.
- `--age` takes units `y`, `m`, `w` and `d`, like `13w2d`, `6m` or `1y3m`. A
  month is a twelfth of a year and a year is 365.25 days.
- `--centile` is between 0 and 100, not inclusive.

Run with `uv run centiles` from a checkout, or use `mise run f` and `mise run m`
to run it with the sex already set:

```sh
mise run f weight --age 13w2d --value 6.1
```

## Development

Tools come from [mise](https://mise.jdx.dev) and Python dependencies from
[uv](https://docs.astral.sh/uv/).

```sh
mise run setup        # install the tools and git hooks
mise run test         # run the tests
mise run fix-all      # apply linter and formatter fixes
mise run check-all    # run all linters, formatters and type checks
```

[hk](https://hk.jdx.dev) runs the checks as a pre-commit hook and checks that
commit messages follow
[conventional commits](https://www.conventionalcommits.org).

This project was created from
[python-template](https://github.com/CallumKerson/python-template). Pull in
template changes with `mise run update-template`.
