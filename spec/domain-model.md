# Domain model (v1 scope)

Three modules: Patient Administration, Order Comms & Results, Scheduling. Every PAS needs these regardless of specialty; everything else is backlog (`data-dictionary.md` has the full legacy module census, `roadmap.md` has the sequencing).

## Design principles

- **Every persistent entity carries an audit trail**: who created it and when, who last changed it and when. Clinical and administrative records are never silently overwritten without a trace.
- **State changes are history, not just a current value.** Anywhere an entity has a meaningful status (an episode, an order, an appointment), model the full sequence of status changes - what it changed to, when, by whom, and why - not just the current status column. This was one of the few genuinely good patterns in the legacy system, and it matters for clinical governance and dispute resolution.
- **Soft deletion only.** Clinical and administrative records are deactivated, never hard-deleted. A PAS is also a legal record.
- **Identifiers are opaque (UUIDs), not sequential integers.** Sequential IDs leak record counts and creation order, which matters for a system holding sensitive health data, and opaque identifiers make future multi-system or offline sync scenarios easier.
- **Coded fields bind to a controlled vocabulary**, preferably an existing standard (SNOMED CT, the NHS Data Dictionary's national code sets) rather than a bespoke local list, wherever a suitable standard exists. Reserve local lookups for genuinely local choices, such as a trust's own ward names. [`sct`](https://github.com/pacharanero/sct) - a local-first SNOMED CT toolchain, no network calls - is a strong candidate for the terminology side of this: `sct serve` runs a FHIR R4 terminology server, and it may be usable directly wherever openpas needs one (Helios's own `HFS_TERMINOLOGY_SERVER` setting expects exactly this shape) rather than standing up something separate. Worth a concrete spike once Phase 1 needs real coded fields, not assumed blindly now.
- **FHIR UK Core alignment where there's a clean match**, so the system can exchange data with the rest of the NHS ecosystem without a bespoke integration for every partner. Where UK policy requirements have no FHIR equivalent - the RTT pathway clock, for one - model them as first-class openpas concepts anyway; the absence of a standard doesn't make the requirement optional.
- **Placer/filler identifier pairs on anything ordered externally**, because every real lab, imaging, or pharmacy system a trust integrates with expects HL7v2 or FHIR messaging built around that distinction.

## Module 1: Patient Administration

### Patient

The person receiving or registered for care.

Key attributes: name (with history, since patients change names), date of birth, NHS Number (with verification status, separate from "has a number"), hospital/local identifiers, address and contact details (with history), registered GP and practice, next of kin, active/merged status - patient records get merged when duplicates are found, and a merge is a recorded event, not a silent delete. Structure demographics and identifiers per [NHS Data Dictionary](https://www.datadictionary.nhs.uk/) standards wherever one exists for a field. Sex is administrative and tracked with history rather than overwritten - a sex change is a recorded event, not a correction. Gender identity is a separate, optional field from sex; the two answer different questions and shouldn't be conflated into one.

Relationships: has many Episodes of Care over time; has one current GP and practice, referenced by their [NHS Organisation Data Service](https://www.odsdatasearchandexport.nhs.uk/referenceDataCatalogue/ODS-Data-Model_571324843.html) code rather than free text - [`ods`](https://github.com/olizilla/ods) publishes exactly this data as queryable Parquet files (`ods find`, `ods info`), a natural fit for resolving and validating these references; may have a next of kin and a linked associated record if merged.

### Episode of Care

One continuous relationship between a patient and the provider, from first contact to final discharge - matching FHIR's `EpisodeOfCare` resource directly, rather than inventing a separate "spell" concept on top of it. Can span specialty changes; track those as history on the episode itself, not as a reason to end it and start a new one.

Key attributes: specialty (with history, since it can change mid-episode), responsible clinician, start date, end date, status history, the Referral that opened it.

Relationships: belongs to one Patient; opened by one Referral; for inpatients, associated with one or more Admissions.

### Admission

A single inpatient stay, start to discharge. Ward and bed moves within it are a location history on the admission, not separate entities - this matches how FHIR's `Encounter.location` already works, and avoids a layer FHIR doesn't need.

Key attributes: admission date/time, expected and actual discharge date/time, admission type (elective, emergency, transfer), ward/bed location history, on-leave status.

Relationships: belongs to one Episode of Care.

### Referral

The request that opens an Episode of Care.

Key attributes: source (GP, consultant, self), received date, specialty requested, priority, status history (received, triaged, accepted, rejected).

Relationships: opens one Episode of Care.

**Lifecycle**: Referral received -> Episode of Care opened under a specialty -> (if inpatient) one or more Admissions, each with its own ward/bed location history -> Episode of Care closed.

## Module 2: Order Comms & Results

### Order (Service Request)

A single requested investigation, test, or procedure.

Key attributes: requested item (from a catalogue - pathology test, imaging study, procedure), requesting clinician, requested location, priority, placer order number (assigned by openpas), filler order number (assigned by the fulfilling system, once known), status history (requested, in progress, resulted, cancelled).

Relationships: belongs to one Order Session; linked to one or more Specimens where a physical sample is needed; produces one Result once complete.

### Order Session

The header for one ordering action, bundling the Orders placed together in one sitting.

Key attributes: ordering clinician, ordering location, date/time.

Relationships: contains one or more Orders.

### Specimen

The physical sample collected to satisfy one or more Orders.

Key attributes: collection site, specimen source/type, collection date/time, received date/time, collection status history (awaiting collection, collected, received).

Relationships: satisfies one or more Orders, many-to-many - one specimen can answer several tests from a single blood draw, and one order can need several specimens.

### Result (Observation)

The answer to an Order. Results come in three shapes, and the model keeps them distinct rather than forcing everything into one free-text field:

- structured (discrete values with units and reference ranges - most pathology),
- narrative (free text, typical of radiology and clinical reports),
- document (an attached file - a scanned report or external PDF).

Key attributes: result type (structured/narrative/document), reported date/time, reporting clinician, review status - a result can be resulted but not yet reviewed by the requesting clinician, which is a distinct, trackable state.

Relationships: answers one Order.

**Lifecycle**: Order placed (within an Order Session) -> Specimen collected if needed -> Specimen received by the fulfilling service -> Result recorded -> Result reviewed by the requesting clinician.

**Open question**: who actions a result, on what timescale, and what happens if the responsible clinician is unavailable - secondary reviewers, escalation paths - isn't decided yet. See `queries.md`.

## Module 3: Scheduling

### Session / Slot

A block of bookable time for a service or clinician: a clinic session, a theatre list.

Key attributes: service, location, clinician, date, start/end time, slot capacity.

Relationships: contains one or more Appointments.

### Appointment

A scheduled contact between a patient and a service or clinician.

Key attributes: patient, service/clinician, date/time, status history (booked, confirmed, arrived, seen, completed, cancelled, did-not-attend), outcome and outcome reason - "did not attend" and "attended, no further action needed" are both terminal but very different.

Relationships: booked into one Session/Slot; for a given patient, linked to the Episode of Care or Referral it serves; may itself contribute to an RTT Pathway clock.

### RTT Pathway

NHS England's statutory Referral-To-Treatment waiting-time pathway: the clock a Referral starts, that Appointments and treatment events can pause, reset, or stop. This is a UK policy requirement with real reporting obligations attached (trusts report breaches), not an optional nice-to-have.

Key attributes: clock start date (from the Referral), clock stop date (first definitive treatment), pause periods and reasons, breach status.

Relationships: started by one Referral; affected by any number of Appointments and treatment events along that Referral's pathway.

**Lifecycle**: Session/Slot created -> Appointment booked into it -> patient arrives -> patient seen -> outcome recorded, which may affect the RTT Pathway clock.

**Open question**: the actual compliance-reporting mechanics - required metrics, reporting frequency, what counts as a breach or exception, and how the data gets exported - aren't decided yet. Whatever it is must be seamless for end users, not a manual chore bolted on afterward. See `queries.md`.

## Out of scope for v1

Everything else openMAXIMS covered - nursing assessments, broader clinical notes/observations beyond OCRR, emergency/A&E-specific workflow, oncology, therapies, spinal injuries, care of the elderly, integrated care pathways, correspondence/letters, billing, and HL7 interfacing as a dedicated module - is real, documented scope for later (see `data-dictionary.md` for the legacy module census and `roadmap.md` for sequencing), not detailed here yet.
