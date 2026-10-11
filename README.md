# openpas

A specification and roadmap for a modern, open-source Patient Administration System (PAS) / order-comms / EPR core.

## What is this?

A clean-room domain specification, informed by (not copied from) the retired [openMAXIMS](https://github.com/IMS-MAXIMS/openMAXIMS) system - a decade-old UK NHS PAS/order-comms/EPR that's open source (AGPLv3) but practically unmaintainable without its vendor's proprietary IDE. openpas mines openMAXIMS's database schema and Hibernate mappings for domain facts - what entities and relationships a working PAS/OCRR actually has to represent - then specifies an independent, modern system from scratch.

## What is it not?

Not a fork of openMAXIMS, not a port, not runnable software. There is no code here yet - see `spec/roadmap.md` for what happens before any gets written.

## Who is it for?

Right now: whoever picks up the open queries in `spec/queries.md` and starts Phase 1 of the roadmap. Eventually, if it goes that far: NHS trusts and other healthcare providers who want a PAS core they can run, inspect, and modify without a per-seat vendor licence.

## Structure

- [`spec/`](spec/README.md) - the actual specification: domain model, ubiquitous language, data dictionary, roadmap, and open decisions. Start at `spec/README.md`.
- [`reference/`](reference/README.md) - the mined legacy facts (schema statistics, extraction script) that informed the spec, kept separate on purpose. Not required reading to use the spec; required reading to audit where it came from.

## Status

Specification only, as of October 2026. Nothing here has been implemented or reviewed by anyone other than the person who wrote it.

## Licensing

Written content in this repository follows house style and is CC-BY-SA-4.0. A code licence has not been chosen yet - this is a documentation-only repository so far; see `spec/queries.md` for the open licence and copyright-holder decisions that need resolving before any code or a `LICENSE` file is added. Mined data under `reference/` was extracted from openMAXIMS, which is AGPLv3, copyright IMS MAXIMS - see `reference/README.md` for the provenance detail and why it isn't just reused directly.
