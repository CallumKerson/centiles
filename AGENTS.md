# AGENTS.md

A CLI for everything to do with baby measurement percentiles.

The `centiles` CLI is built with
[cyclopts](https://cyclopts.readthedocs.io).

## Commands

- `mise run test` (or `uv run pytest`) runs the tests.
- `mise run check-all` runs all linters, formatters and type checks through hk;
  `mise run fix-all` applies fixes. Run both checks and tests before
  committing.
- `uv run centiles <args>` runs the CLI; `mise run f <args>` and
  `mise run m <args>` run it with `CENTILES_SEX` set.
- `mise run build` builds the sdist and wheel into `dist/`.

## Layout

- `src/centiles/cli.py`: the cyclopts `app`, exposed as the
  `centiles` script in `pyproject.toml`. The `weight`, `height` and `head`
  commands each take `--sex` (or `CENTILES_SEX`) plus two of `--age`, `--value`
  and `--centile`, and share `_run` to print the third. With `--compare`, they
  take `--age` and `--centile` and print the age at which the comparison
  centile reaches the same value.
- `src/centiles/growth.py`: plain functions over
  [rcpchgrowth](https://github.com/rcpch/rcpchgrowth-python) and its UK-WHO
  reference: age parsing and formatting, `centile_for`, `value_for`, and
  `age_for` (which bisects over age), and `matching_age`, which chains
  `value_for` and `age_for` for `--compare`. They raise `ValueError` for bad
  input, which the CLI prints as an error.
- `tests/`: pytest tests. Call `app([...])` with CLI tokens; it exits via
  `SystemExit`, so wrap calls in `pytest.raises(SystemExit)`.

## Tooling

- mise manages tools; uv manages Python and dependencies, and mise runs
  `uv sync` before tasks when they're out of date.
- Tool config lives in `.config/`, apart from `.typos.toml`, as typos only
  reads config from the root.
- hk (`.config/hk.pkl`) runs ruff, ty, rumdl (Markdown), ryl (YAML), tombi
  (TOML), typos and file hygiene checks as a pre-commit hook, and checks
  commit messages follow conventional commits.
- GitHub Actions runs `mise run ci` on pull requests and `main`. On `main`,
  release-please runs once CI passes and makes releases from conventional
  commits, and the Release Artefacts workflow attaches each release's sdist
  and wheel.
- Renovate updates dependencies using the shared
  [CallumKerson/renovate-config](https://github.com/CallumKerson/renovate-config)
  preset, pinned to a release in `.github/renovate.json`. Put rules only this
  repo needs there too.
- jactionlint rejects actions not pinned to a commit. Pin new ones with
  `mise exec pinact@latest -- pinact run`.
- Tool and dependency versions belong to this repo. The
  [python-template](https://github.com/CallumKerson/python-template) copier
  template only adds new tools unversioned; `mise run update-template` pulls
  template changes and `mise run pin` pins anything new.
