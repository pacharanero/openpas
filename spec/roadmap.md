# Roadmap

Legend: `[x]` done, `[~]` in progress, `[ ]` not started. `R` identifiers are stable and are not reused; see [house style's specs.md](https://github.com/pacharanero/house-style/blob/main/specs.md) for the convention.

## Standing principles

- Ship Patient Administration, Order Comms & Results, and Scheduling as a coherent v1 before adding any specialty module. A PAS that can't register a patient, place an order, and book an appointment isn't a PAS yet, regardless of how many specialty forms it has.
- Every module after v1 gets the same treatment as v1 did: mine the legacy census in `data-dictionary.md` for scale and field detail, then write an independent spec before any code.
- No real patient data, anywhere, until `SAFETY.md`'s medical-device and clinical-safety questions are actually answered, not just scaffolded.

## Phase 0: Foundations

- [x] **R1 - Mine the legacy schema.** DDL + Hibernate mappings from openMAXIMS parsed into `reference/legacy-schema/` (1,371 tables, 17,283 columns, 2,740 foreign keys) via `reference/scripts/extract-ddl-schema.py`.
- [x] **R2 - Write the v1 domain model.** `domain-model.md` and `ubiquitous-language.md` cover Patient Administration, Order Comms & Results, and Scheduling.
- [x] **R3 - Resolve open spec queries.** Answered in `queries.md`: copyright holder pacharanero/Marcus Baw for now (transfer to the planned Open Health Technology Foundation later), licence AGPL-3.0-or-later, stack Rust + PostgreSQL, full FHIR as the system of record, backend/client separation from the first commit, name `openpas`.
- [ ] **R4 - Independent review of the domain model.** Have someone with clinical/health-informatics background (not just the person who mined it) sanity-check `domain-model.md` and `ubiquitous-language.md` against real PAS/OCRR workflow, not just against the legacy schema.
- [x] **R5 - Decide the FHIR alignment strategy.** Full FHIR, per `queries.md` query 5. Recorded as [`adr/0001-fhir-foundation.md`](../adr/0001-fhir-foundation.md): build on the Helios FHIR Server crates (MIT-licensed, Rust-native, Postgres-backed) rather than writing FHIR server machinery from scratch.
- [x] **R6 - Bootstrap the implementation repo.** Cargo workspace, `crates/backend`, licence file, CI, scaffold per house style `new-repos.md` - done in place inside this `openpas/` directory rather than a separate repo.
- [x] **R28 - Wire up the Helios FHIR Server crates against PostgreSQL.** `crates/backend` depends directly on `helios-fhir`/`helios-persistence`/`helios-rest` 0.2.4; verified end to end against a real PostgreSQL instance, not just compiled. See `adr/0001-fhir-foundation.md` for why, `crates/backend/README.md` for how.

## Phase 1: Patient Administration MVP

Per `adr/0001-fhir-foundation.md`, these land as FHIR resources (`Patient`, `EpisodeOfCare`, `Encounter`) on the Helios/Postgres substrate from R28, plus openpas-specific extensions and custom operations where FHIR has no equivalent concept.

- [~] **R7 - Patient registration and demographics.** Register, list, detail, edit, conservative link-only merge, and address/contact history are live end to end in `crates/web`, including live NHS Number checksum validation (`POST /openpas/nhs-number/validate`, via the [`nhs-number`](https://crates.io/crates/nhs-number) crate - pinned to 0.3.0, see `crates/backend/Cargo.toml` for why) before a `Patient` is created or updated. Not done: name history, administrative sex history, gender identity, GP/practice, next of kin, local identifiers, and an audited merge-reversal workflow.
- [ ] **R8 - Episode of Care lifecycle.** Open, transfer specialty, close.
- [ ] **R9 - Admission / bed management.** Admit, transfer, discharge.
- [ ] **R10 - Referral intake.**
- [ ] **R11 - Conformance fixtures for Patient Administration.** Golden test cases per house style `specs.md`, covering the lifecycle in R7-R10.
- [x] **R29 - First web UI thin slice.** `crates/web`: axum + Askama + htmx + tower-sessions, a separate process/client per `adr/0002-web-ui.md`. Login (seeded demo credential only, not user management), patient registration with live NHS Number validation, and a patient list - the thinnest slice that proves browser-to-UI-to-backend-to-FHIR-to-Postgres works end to end. Visually verified (desktop/mobile viewports, keyboard-only navigation, error/success states) per house style `ui.md`, not just compiled.
- [x] **R30 - Responsive dashboard shell and patient editing.** DaisyUI/Tailwind CSS dashboard shell using the official responsive drawer pattern, full-width working pages, mobile navigation with keyboard/Escape handling, and FHIR Patient editing proven through the backend and UI against real Postgres. Verified at phone/tablet/desktop widths, reduced motion, and 200% zoom.
- [x] **R31 - Conservative patient merge.** Explicit survivor/duplicate selection, no automatic field movement, reciprocal FHIR `replaces` / `replaced-by` links, duplicate deactivation, optimistic version checks, and both writes committed atomically through a FHIR transaction Bundle. Merged records remain visible but cannot be edited or merged again through the UI.
- [x] **R32 - Address and contact history.** Optional home address, phone, and email on patient registration/editing; changed or cleared values remain as ended FHIR `Address` / `ContactPoint` entries, while unrelated entries from other clients are preserved. A responsive patient detail page exposes current and prior values. Updates use FHIR `If-Match` to reject concurrent changes rather than overwrite them.

## Phase 2: Order Comms & Results MVP

- [ ] **R12 - Order placement.** Service Request creation with placer/filler identifier handling.
- [ ] **R13 - Specimen collection and tracking.**
- [ ] **R14 - Result capture and review.** Structured, narrative, and document result types; review/sign-off workflow for the requesting clinician.
- [ ] **R15 - Interoperability compatibility review.** HL7v2 ORM/ORU or FHIR messaging, since any real deployment has to talk to existing lab/imaging systems that already speak one of those.

## Phase 3: Scheduling MVP

- [ ] **R16 - Clinic session/slot management.**
- [ ] **R17 - Appointment booking and status lifecycle.** Booked, confirmed, arrived, seen, DNA, cancelled.
- [ ] **R18 - RTT 18-week pathway clock tracking and breach reporting.**

## Phase 4: Expand domain coverage

Sequencing below is a starting signal from legacy table-count scale (`data-dictionary.md`'s module census), not a commitment - revisit priority once v1 is live and real usage patterns exist.

- [ ] **R19 - Nursing assessments module spec.** Legacy `ns_`/`care_`-adjacent scope, ~150 tables.
- [ ] **R20 - Broader clinical notes/observations module spec.** Beyond what OCRR already covers.
- [ ] **R21 - Emergency/A&E module spec.** Legacy `emer_`, 66 tables.
- [ ] **R22 - Correspondence module spec.** Clinic letters, discharge summaries.
- [ ] **R23 - Billing module spec.**
- [ ] **R24 - Remaining specialty modules.** Oncology, therapies, spinal injuries, care of the elderly, RACPC, integrated care pathways - lowest priority, revisit once core is live.

## Phase 5: Non-functional and governance (ongoing)

- [ ] **R25 - Build out the clinical safety case.** `SAFETY.md` to a full safety file set once real functionality exists, per house style `clinical-safety.md`. Depends on R7 or later landing first - there's no clinical behaviour to document a safety case for yet.
- [ ] **R26 - Data migration strategy.** Only if openpas ever targets migrating real data from a live openMAXIMS or other legacy PAS instance. Flagged explicitly as high-risk, needing separate legal and clinical-safety review, not a simple ETL job.
- [ ] **R27 - Interoperability conformance suite.** FHIR UK Core / HL7v2 compatibility tests, per house style `specs.md`'s conformance-suite pattern.
- [ ] **R33 - Replace `dtolnay/rust-toolchain` with plain rustup.** Quick, zero-risk, do first: `dtolnay/rust-toolchain` has no versioned releases - its `v1` tag is a permanently floating alias the maintainer force-moves to track `master`, so SHA-pinning it (the current house-style convention before this fix) only buys time until it drifts again and trips zizmor's `ref-version-mismatch` check. This is documented upstream behaviour, not a bug: see [dtolnay/rust-toolchain#180](https://github.com/dtolnay/rust-toolchain/issues/180), open since March 2026, unaddressed. New house style (`ci.md`, "Rust toolchain setup"): a committed `rust-toolchain.toml` plus `rustup toolchain install --no-self-update` in CI, no Action at all. Worked reference: `pacharanero/clincalc` commit `9f00f57`.
