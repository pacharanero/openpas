# Reference: mined legacy facts

Raw facts mined from openMAXIMS, kept separate from `../spec/` on purpose. Read `../spec/README.md`'s "clean-room stance" section first for why.

## What's here

- `scripts/extract-ddl-schema.py` - a script written for this project that parses an openMAXIMS DDL dump into structured data. Original tool, not legacy-derived.
- `legacy-schema/legacy-schema.json` - its output: one record per table (name, module prefix, columns with type/nullability, primary key, foreign keys). 1,371 tables, 17,283 columns, 2,740 foreign keys.
- `legacy-schema/legacy-schema-summary.md` - a generated per-module rollup of the same data. Regenerate, don't hand-edit.

## Provenance

Mined from `DDL-scripts-ref147-openMAXIMS.sql`, a 65,073-line T-SQL dump shipped in the official [`IMS-MAXIMS/openMAXIMS`](https://github.com/IMS-MAXIMS/openMAXIMS) repository's `Source Library/database/` directory (release ref147, part of openMAXIMS 10.5.x). That file is itself generated output: openMAXIMS's own `README - Further details.txt` states the schema is produced from a UML-like meta-model by IMS MAXIMS's proprietary "IMS Development Environment" code generator, then exported to T-SQL. It is not vendored into this repository - see "Licence" below for why - so the script takes its path as an argument. The exact invocation used to produce the current `legacy-schema/` output:

```sh
python3 scripts/extract-ddl-schema.py \
  "<path-to-openMAXIMS-checkout>/Source Library/database/DDL-scripts-ref147-openMAXIMS.sql" \
  legacy-schema
```

Semantic grounding for `../spec/domain-model.md` and `../spec/ubiquitous-language.md` (entity relationships, the placer/filler order number pattern, the polymorphic Result shape, the RTT pathway fields) additionally came from manual inspection of specific Hibernate mapping files in the same openMAXIMS checkout, not from a generalised parser: `Patient.hbm.xml`, `EpisodeOfCare.hbm.xml`, `CareSpell.hbm.xml`, `InpatientEpisode.hbm.xml`, `WardStay.hbm.xml`, `Referral.hbm.xml`, `OrderInvestigation.hbm.xml`, `OcsOrderSession.hbm.xml`, `OrderSpecimen.hbm.xml`, `ResultDetails.hbm.xml`, `Booking_Appointment.hbm.xml`, `Sch_Session.hbm.xml`, `Session_Slot.hbm.xml`, and the `PathwayClock`/`PathwaysRTTClockImpact` pair, all under `DomainObjects/src/ims/**/domain/objects/`.

The sibling `openmaxims-linux` fork was also reviewed but contributes nothing beyond what's in upstream openMAXIMS - it stalled after five days in June 2014, one commit short of a bug upstream fixed the following day - so it is not separately cited as a source.

## Licence

The source DDL file is AGPLv3, copyright IMS MAXIMS. The extracted `legacy-schema.json`/`.md` in this directory are a mechanical, lossy listing of table/column/foreign-key names and types - facts with thin copyright protection at best - but this project treats them as carrying the source's AGPLv3 obligations out of caution, and does not redistribute the source file itself.

**Do not copy structures from this directory into `../spec/` prose or into any future implementation code or migrations.** Use it only to: cross-check the new spec's completeness against what a working PAS/OCRR had to handle, recall field-level detail when a backlog module gets its turn, and size backlog modules for roadmap prioritisation by table/column count. The spec itself must stay independently authored - see `../spec/README.md`.
