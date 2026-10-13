# Agent instructions

openpas follows [Marcus Baw's house style](https://github.com/pacharanero/house-style). Read the relevant standard there before doing that kind of work (CI, dependencies, licensing, testing, commits), it is the source of truth unless this file documents a deliberate exception.

## Read first

1. [`spec/README.md`](spec/README.md) - domain specification, reading order, clean-room stance.
2. [`adr/`](adr/) - architecture decisions: [`0001-fhir-foundation.md`](adr/0001-fhir-foundation.md) (backend), [`0002-web-ui.md`](adr/0002-web-ui.md) (UI, access control).
3. [`spec/roadmap.md`](spec/roadmap.md) - what's done, what's next, stable `R`-numbered items.

## Invariants

- Nothing under `spec/` copies legacy table/column structure verbatim from `reference/` - see `spec/README.md`'s clean-room stance. This matters for licensing freedom, not just style.
- `crates/backend` is the backend; it owns no UI. `crates/web` is a client - a separate process talking to it over HTTP, per ADR-0001/0002. Future clients (mobile, CLI) follow the same pattern.
- `crates/web`'s login is a single seeded demo credential, not user management, and the FHIR backend trusts it with no auth of its own yet - see ADR-0002's access-control section before either changes.
- No real patient data anywhere until `SAFETY.md`'s medical-device and clinical-safety questions are answered, not just scaffolded.

## Validation commands

- `s/fmt` - `cargo fmt --all`
- `s/lint` - `cargo clippy --all-targets -- -D warnings`
- `s/test` - `cargo test`
- `s/css` - compile Tailwind CSS + DaisyUI into the checked-in web stylesheet; requires `npm ci --ignore-scripts` after cloning
- `s/check` - Rust checks plus npm audit and a reproducible web-asset build; run before every commit

## Local git workflow

Solo, fast-moving repository: commit and push directly to `main` in coherent parcels, per house style `commits.md`. Remote is `pacharanero/openpas` on GitHub (private). CI (`fmt`/`clippy`/`test`/`audit`/`reuse`/`workflow-security`) runs on push and PR - see `.github/workflows/ci.yml`.
