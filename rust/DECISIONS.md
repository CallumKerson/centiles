# Decisions

## ADR-001: Port the whole CLI

Status: accepted (2026-11-26)

Context: `centiles` is a CLI with one entry point, `centiles.cli:app`, and three
commands.

Decision: Port every command (`weight`, `height`, `head`, and `--compare`) into
one Rust binary.

Consequences: The Python package stays as reference only.

## ADR-002: clap replaces cyclopts

Status: accepted (2026-11-26)

Context: No crate reproduces cyclopts; clap is the usual Rust choice.

Decision: Parse arguments with clap's builder API in `rust/app/src/cli.rs`.

Consequences: See ADR-004 for how its output differs.

## ADR-003: Port only the UK-WHO parts of rcpchgrowth

Status: accepted (2026-11-26)

Context: `growth.py` calls only `sds_for_measurement` and `measurement_from_sds`
on the UK-WHO reference. rcpchgrowth is 3.3 MB and covers many other references.

Decision: Port those two functions for UK-WHO weight, height and head
circumference, with the four source tables (UK90 preterm, WHO infants, WHO
children, UK90 child) extracted into `centiles-rs/data/uk_who.json` and built
into the binary. Keep the library's edge behaviour, including its `IndexError`
and SciPy range error at the gap around two years, and NumPy arithmetic where
SciPy interpolates linearly. Use the C library's `erfc`, as Python does.

Consequences: Results match the original to the bit. The tables are licensed
under the AGPL, like the original.

## ADR-004: Help, usage errors and bad option values are clap's

Status: accepted (2026-11-26)

Context: cyclopts draws help and parse errors in Rich boxes. Reproducing them
would mean a box renderer that only mimics another library.

Decision: Print clap's help, version handling and usage errors (exit code 2),
except `--version`, which prints the bare version as the original did. Every
case where the original printed a cyclopts box was re-recorded from the Rust
build; every other case, including every result and every `Error:` line from
valid data, is the original's.

Consequences: 127 cases in `app-cases` hold clap's output; they're listed in
`app-cases-rebased.txt`. Bare `centiles` now exits 2 with help, where cyclopts
exited 0.

## ADR-005: Errors the original crashed on print as `Error:`

Status: accepted (2026-11-26)

Context: For `--value 0` or a negative value the original raised
`ZeroDivisionError` or a complex-number `TypeError` past its `ValueError`
handler and printed a traceback.

Decision: Print `Error: <message>` with exit code 1, as for every other error.

Consequences: The 2 traceback cases were re-recorded from the Rust build (listed
in `app-cases-rebased.txt`). The underlying bugs stay, matching the original.

## ADR-006: Unit cases use type-valid inputs

Status: accepted (2026-11-26)

Context: `record`'s generated inputs fill `Literal["weight", ...]` parameters
with lists, dicts and `None`; 80% of them failed on Python type errors the CLI
can never reach.

Decision: Record `centiles-rs` with `--count 0`, the test suite's calls and
`centiles-rs/inputs.jsonl`: every valid measurement and sex over edge ages,
centiles and values, plus age strings with Unicode traps.

Consequences: 6,840 unit cases, all reachable from the CLI.
