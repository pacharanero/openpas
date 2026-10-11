# openpas specification

openpas is a clean-room specification and roadmap for a modern, open-source Patient Administration System (PAS) / order-comms / EPR core. It is informed by, but not copied from, the retired [openMAXIMS](https://github.com/IMS-MAXIMS/openMAXIMS) system (IMS MAXIMS, AGPLv3, live at Taunton and Somerset NHS Trust circa 2014-2015) - see [`../reference/README.md`](../reference/README.md) for exactly what was mined and why.

## Status

Specification only. No implementation exists yet. Treat everything here as a first pass: informed by real legacy domain facts, but independently authored and open to revision as the decisions in `queries.md` get resolved.

## Reading order

1. [ubiquitous-language.md](ubiquitous-language.md) - canonical terms, so "episode" and "encounter" mean one thing throughout.
2. [domain-model.md](domain-model.md) - the actual specification: entities, relationships, and lifecycles for the v1 scope.
3. [data-dictionary.md](data-dictionary.md) - concrete attributes for the core entities, plus a full census of the legacy modules not yet in scope.
4. [roadmap.md](roadmap.md) - phased plan, `R`-numbered, with status checkboxes.
5. [queries.md](queries.md) - decisions that need a human (licence, copyright holder, tech stack) before implementation can start.

## Scope for v1

Every PAS deployment needs three things regardless of specialty: patient administration (registration, episodes, admissions), order comms and results (OCRR), and scheduling (appointments). Those three are specified in full in `domain-model.md`. Everything else openMAXIMS did - oncology, therapies, nursing assessments, emergency, pathways, correspondence, billing, and a dozen smaller specialty modules - is catalogued in `data-dictionary.md`'s legacy module census and sequenced as backlog in `roadmap.md`, not detailed yet.

## Clean-room stance

openMAXIMS is already open source (AGPLv3), so there's no legal-access problem to solve here. The reason to specify independently rather than fork or copy its schema is licensing freedom: openpas's own licence is still an open question (`queries.md`), and copying openMAXIMS's table/column structure verbatim would drag its AGPLv3 copyleft along regardless of what licence openpas later picks. So the rule for everything under `spec/` is: read the legacy system for facts about what a working PAS/OCRR had to represent, then write our own entities, names, and structure. The raw mined facts live under [`../reference/`](../reference/), kept deliberately separate, and are reference material for traceability and completeness-checking, not a drafting template.
