# ADR-0001: Adopt Helios FHIR Server crates as the FHIR foundation

Status: Accepted

Date: 2026-10-11

## Context

`spec/queries.md` resolved several foundational decisions: Rust, PostgreSQL, and "go full FHIR for the system of record... it will be a huge win to be able to say everything in the whole PAS is already FHIR-compliant" (query 5), with backend and clients cleanly separated from the first commit.

Rust's FHIR tooling was an open risk going into this: the mature FHIR server implementations (HAPI FHIR, Firely, Google/Microsoft's FHIR servers) are Java, .NET, or Go, not Rust. Writing a spec-conformant FHIR RESTful API (CRUD, search, versioning, conditional operations, Bundle transactions) from scratch is a multi-year undertaking in any language, and committing to "full FHIR" while also committing to the language with the thinnest FHIR ecosystem looked like it could be a bad combination.

Researching the current (October 2026) Rust FHIR ecosystem for this ADR found [`HeliosSoftware/hfs`](https://github.com/HeliosSoftware/hfs), the Helios FHIR Server:

- A workspace of crates (`helios-fhir`, `helios-persistence`, `helios-rest`, `helios-auth`, `helios-audit`, `helios-cds-hooks`, and others), version 0.2.4 at time of writing.
- MIT licensed, so no licence-compatibility problem for an AGPL-3.0-or-later project.
- Actively maintained: the crates used here were last updated within the last ten days of this decision.
- `helios-fhir` provides the complete FHIR R4/R4B/R5/R6 data model as serde-serializable Rust types, generated from the official HL7 StructureDefinitions, gated behind Cargo features (`R4`, `R4B`, `R5`, `R6`).
- `helios-persistence` provides a `Backend`/`ResourceStorage` trait hierarchy with concrete implementations for SQLite, **PostgreSQL**, MongoDB, Elasticsearch, and S3.
- `helios-rest` provides `create_app_with_config`, which turns a `Backend` into a complete Axum `Router` implementing the FHIR RESTful API: CRUD, search with chained parameters, history/versioning, batch/transaction Bundles, bulk data export, and more.
- A full standalone server binary (`hfs`) also exists, built from the same crates, for anyone who wants to run FHIR storage as an off-the-shelf service rather than a library dependency.

This is a materially different risk profile than "build a FHIR server from scratch in Rust." Verification for this ADR included actually compiling code against `helios-fhir` 0.2.4 (catching a bug in the crate's own README example in the process - primitive fields are `Element<T, Extension>`, not bare `T`, and need `.into()`), and confirming `helios-persistence`'s `PostgresBackend::new(PostgresConfig::default())` and `helios-rest`'s `create_app_with_config` each work from their own documented, independently-verified examples. Combining `PostgresBackend` with `create_app_with_config` specifically was not found as a single documented example anywhere - that combination needed first-hand verification, done as part of implementing this decision (see `crates/backend`).

## Decision

Build `openpas-backend` directly on top of the Helios FHIR Server crates rather than writing FHIR server machinery from scratch:

- `helios-fhir` (feature `R4`) for the FHIR resource type system.
- `helios-persistence`'s `PostgresBackend` for storage, matching the PostgreSQL decision already made.
- `helios-rest`'s `create_app_with_config` for the FHIR RESTful API surface (CRUD, search, versioning, Bundles) as an Axum `Router`.

openpas's own code is the PAS-specific domain and workflow layer on top: Episodes of Care, Admissions, RTT pathway tracking, placer/filler order workflows, and the rest of `spec/domain-model.md` - realised as FHIR resources (`EpisodeOfCare`, `Encounter`, `ServiceRequest`, `Specimen`, `Observation`, `Appointment`, and so on) plus openpas-specific extensions and custom operations where FHIR has no equivalent (the RTT pathway clock has none, per `spec/ubiquitous-language.md`). That domain layer is `spec/roadmap.md` Phase 1 onward, not this ADR.

`crates/backend` is the first workspace member and is explicitly the **backend** side of the backend/client split: it exposes an HTTP API and owns no UI. Future clients (web, mobile, CLI) are separate workspace members added when there is an actual client to build, not stubbed out empty now.

## Consequences

**Easier:**

- FHIR spec conformance (search, versioning, Bundle transactions, bulk export) comes from a maintained upstream rather than years of in-house work.
- The "everything is already FHIR-compliant" goal from `queries.md` is close to free for the baseline CRUD/search surface; openpas's engineering effort concentrates on genuinely novel PAS workflow logic instead of re-deriving FHIR plumbing.
- `helios-cds-hooks` exists already if openpas ever wants CDS Hooks-based clinical decision support, without that being a reason to choose the stack now.

**Harder / riskier:**

- openpas now has a real, specific dependency on a single third-party project (not a standards body or a langauge-ecosystem default). If Helios stalls or changes direction, the exit cost is real: `helios-fhir`'s types are the easiest part to keep (simple, generated, swappable), but `helios-persistence`/`helios-rest` are load-bearing infrastructure. Mitigate by keeping PAS-specific code behind our own types where practical, and by revisiting this ADR if Helios's maintenance signal changes - see `dependencies.md`'s exit-cost guidance.
- Helios is 0.2.4: pre-1.0, API can still move. Pin exact versions (see `crates/backend/Cargo.toml`) and treat upgrades as reviewed work, not automatic.
- Helios's own documentation is written overwhelmingly for running the prebuilt `hfs` binary via environment variables; embedding its crates as a library, which is what openpas needs to add custom PAS operations, is a thinner, less-trodden path. Expect to read source, not just docs, as openpas's custom operations grow.
- "Full FHIR for the system of record" via a third-party crate rather than a hand-rolled schema means openpas inherits FHIR's own data model assumptions (resource shapes, reference semantics) wherever they don't cleanly match a PAS/OCRR concept - `spec/ubiquitous-language.md` already flags Order Session as one such mismatch.

## Alternatives considered

- **Write a FHIR server from scratch in Rust.** Rejected: multi-year undertaking duplicating a huge amount of already-solved, spec-conformance-tested work, for a team of one.
- **Use a mature non-Rust FHIR server (HAPI FHIR on the JVM) and write the PAS logic as a separate Rust service calling it.** Rejected for now: splits the stack across two languages/runtimes from day one, when the Rust-native option turns out to cover the same ground. Worth revisiting only if Helios's gaps (see "harder/riskier" above) turn out to matter more in practice than they appear today.
- **"FHIR-flavoured" bespoke relational schema** (the `domain-model.md`/`data-dictionary.md` option that predates this ADR) that borrows FHIR naming without being a real FHIR server. Superseded by this ADR per the explicit "go full FHIR" decision in `queries.md` query 5 - recorded here rather than silently dropped, since it's a legitimate option that was deliberately not chosen.
