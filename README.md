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

Add `--compare` with an age and centile to find the age at which a child on
another centile has the same value. For a 3 month old boy on the 90th centile
for height, the age at which a boy on the 50th centile is as tall:

```sh
centiles height --sex m --age 3m --centile 90 --compare 50
# Height: 64.05 cm, reached on the 50th centile at 17w 5d (4m 2d)
```

- `--sex` is `male`, `female`, `m` or `f`. Set `CENTILES_SEX` to avoid passing
  it every time; `--sex` still wins when given.
- `--age` takes units `y`, `m`, `w` and `d`, like `13w2d`, `6m` or `1y3m`. A
  month is a twelfth of a year and a year is 365.25 days.
- `--centile` and `--compare` are between 0 and 100, not inclusive.

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

## Releases

[release-please](https://github.com/googleapis/release-please) opens a release
pull request from conventional commits on `main`. It uses a GitHub App's token,
so release pull requests run CI. The repository needs the App's client ID in
the `RELEASE_APP_CLIENT_ID` variable and its private key in the
`RELEASE_APP_PRIVATE_KEY` secret. Each release gets its sdist and wheel
attached, so it can be installed with `gh release download` even while the
repository is private.

This project was created from
[python-template](https://github.com/CallumKerson/python-template). Pull in
template changes with `mise run update-template`.

## Licence

centiles is licensed under the
[GNU Affero General Public License v3.0 or later](LICENSE). It uses
[rcpchgrowth](https://github.com/rcpch/rcpchgrowth-python), which is licensed
under the same terms, for the UK-WHO reference data and calculations.
