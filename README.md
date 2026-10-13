# openpas

A modern, open-source Patient Administration System (PAS) / order-comms / EPR core, built in Rust on a FHIR-native PostgreSQL foundation.

## What is this?

A clean-room implementation informed by (not copied from) the retired [openMAXIMS](https://github.com/IMS-MAXIMS/openMAXIMS) system. openpas mines openMAXIMS's database schema and Hibernate mappings for domain facts, specifies an independent modern domain model, and implements it on the Helios FHIR Server crates with PostgreSQL.

## What is it not?

Not a fork or port of openMAXIMS, and not ready for real patient data. The current proof of concept supports login, patient registration/listing/detail/editing/link-only merge, address/contact history, and NHS Number validation; access control and clinical-safety work remain deliberately incomplete.

## Quick start

```sh
s/up
```

This starts PostgreSQL, the FHIR backend, and the web UI, then opens `http://127.0.0.1:8089/login`. The demonstration credential is `demo` / `openpas-demo`. Do not use real patient data.

## Who is it for?

Right now: whoever picks up the open queries in `spec/queries.md` and starts Phase 1 of the roadmap. Eventually, if it goes that far: NHS trusts and other healthcare providers who want a PAS core they can run, inspect, and modify without a per-seat vendor licence.

## Structure

- [`spec/`](spec/README.md) - the actual specification: domain model, ubiquitous language, data dictionary, roadmap, and open decisions. Start at `spec/README.md`.
- [`reference/`](reference/README.md) - the mined legacy facts (schema statistics, extraction script) that informed the spec, kept separate on purpose. Not required reading to use the spec; required reading to audit where it came from.
- [`crates/backend/`](crates/backend/README.md) - Postgres-backed FHIR API and openpas operations.
- [`crates/web/`](crates/web/README.md) - server-rendered DaisyUI web client.

## Status

Phase 1 is in progress. Core patient registration, detail, editing, conservative merge, address/contact history, and NHS Number validation work end to end; remaining demographics, Episode of Care, Admission, Referral, Order Comms, and Scheduling remain on the roadmap.

## Licensing

Code is AGPL-3.0-or-later. Written content is CC-BY-SA-4.0. Mined reference data remains AGPL-3.0-only, copyright IMS MAXIMS. Vendored htmx is 0BSD; generated CSS includes MIT-licensed DaisyUI and Tailwind CSS. `REUSE.toml` and `LICENSES/` carry the complete machine-readable provenance.
