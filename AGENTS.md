# Agent instructions

openpas follows [Marcus Baw's house style](https://github.com/pacharanero/house-style). Read the relevant standard there before doing that kind of work (CI, dependencies, licensing, testing, commits), it is the source of truth unless this file documents a deliberate exception.

## Read first

1. [`spec/README.md`](spec/README.md) - domain specification, reading order, clean-room stance.
2. [`adr/`](adr/) - architecture decisions, starting with [`0001-fhir-foundation.md`](adr/0001-fhir-foundation.md).
3. [`spec/roadmap.md`](spec/roadmap.md) - what's done, what's next, stable `R`-numbered items.

## Invariants

- Nothing under `spec/` copies legacy table/column structure verbatim from `reference/` - see `spec/README.md`'s clean-room stance. This matters for licensing freedom, not just style.
- `crates/backend` is the backend; it owns no UI. Future clients are separate workspace members, added when there's an actual client to build.
- No real patient data anywhere until `SAFETY.md`'s medical-device and clinical-safety questions are answered, not just scaffolded.

## Validation commands

- `s/fmt` - `cargo fmt --all`
- `s/lint` - `cargo clippy --all-targets -- -D warnings`
- `s/test` - `cargo test`
- `s/check` - all three, in that order; run before every commit

## Local git workflow

Solo, fast-moving repository: commit and push directly to `main` in coherent parcels, per house style `commits.md`. No remote exists yet - do not create one or push without asking first.
