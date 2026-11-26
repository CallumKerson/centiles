# Port map

Goal: `centiles` as a pure Rust CLI, with the Python package kept as reference.
Decisions: [DECISIONS.md](DECISIONS.md), ADR-001 to ADR-006.

## Units

| Unit | Crate | State |
| --- | --- | --- |
| `src/centiles/` | `centiles-rs` | done 2026-11-26: gate passed, 6,840/6,840 cases |
| CLI glue | `app` | done 2026-11-26: lifted, replay 972/972, fallback 0% |

## Notes

- Lifted by hand from `app lift`'s copy: lift refuses a hand-edited `main.rs`
  that still reports fallbacks, and also refuses one that doesn't.
- 129 app cases hold the Rust build's output, not the original's: see ADR-004,
  ADR-005 and `app-cases-rebased.txt`.
- Out of scope: other rcpchgrowth references (ADR-003).
