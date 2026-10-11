# Ubiquitous language

Canonical terms for openpas. Use these in specs, future code, and future UI copy. "Legacy alias" records the openMAXIMS term the concept was mined from, for traceability only - don't use legacy names as the canonical ones, and don't treat the legacy alias as confirmation that the concept has to work the way openMAXIMS implemented it.

## Patient

The person receiving or registered for care.

- Legacy alias: `core_patient` / `Patient` domain object.
- FHIR alignment: `Patient`.

## NHS Number

The UK national patient identifier, a 10-digit number with a checksum. Every patient record should carry one once verified, but registration must work before it's confirmed (newborns, overseas visitors, emergency admissions).

- Legacy alias: `Patient.nhsNumber`, with separate `verified`, `duplicateNHSNum`, and `nhsNumberStatus` fields - the legacy model treats "has an NHS number" and "that number is verified against the national Personal Demographics Service" as distinct facts, which is worth keeping.
- FHIR alignment: `Patient.identifier` where `system = https://fhir.nhs.uk/Id/nhs-number`, with a verification-status extension.

## Hospital Number / MRN

A locally-assigned patient identifier, scoped to one organisation, used before or alongside an NHS Number.

- Legacy alias: `hospNum`, `mRNStatus`.
- FHIR alignment: `Patient.identifier` with a local `system`.

## Episode of Care

One continuous relationship between a patient and a provider, from first contact to final discharge, matching FHIR's `EpisodeOfCare` resource directly. Can span specialty changes, tracked as history on the episode rather than modelled as a separate wrapping concept.

- Legacy alias: `core_carespell` / `CareSpell`, and `core_episofcare` / `EpisodeOfCare` - openMAXIMS modelled these as two nested entities (a spell containing specialty-scoped episodes). openpas merges them into one, matching standard FHIR usage directly instead of inventing a "spell" layer on top of it.
- FHIR alignment: `EpisodeOfCare`.
- Terms to avoid: "Care Spell" - an earlier draft term for this same concept, dropped in favour of the FHIR name.

## Admission

A single inpatient stay, start to discharge. Ward and bed moves within it are a location history on the admission itself, not separate entities.

- Legacy alias: `core_inpat_epis` / `InpatientEpisode` (the admission record), with `core_ward_stay` / `WardStay` as openMAXIMS's separate per-move entity - openpas folds that into one admission's location history instead.
- FHIR alignment: `Encounter`, with `Encounter.location` as the ward/bed history.
- Terms to avoid: "Ward Stay" - an earlier draft term for a single bed placement; the admission as a whole is the canonical entity now.

## Referral

The request that opens an Episode of Care, from a GP, another clinician, or self-referral.

- Legacy alias: `core_referral` / `Referral`, linked from `EpisodeOfCare.currentReferral`.
- FHIR alignment: `ServiceRequest` with a referral-specific category, or UK Core's referral profile.

## Order / Service Request

A single requested investigation, test, or procedure for a patient.

- Legacy alias: `ocrr_orderinvestiga` / `OrderInvestigation`. Carries placer and filler order numbers (`placerOrdNum`, `fillerOrdNum`): the HL7v2 ORC-2/ORC-3 concept of "the number the ordering system used" versus "the number the fulfilling system used." Keep this distinction - any real deployment has to interoperate with lab/imaging systems that speak HL7v2 or FHIR and expect both.
- FHIR alignment: `ServiceRequest`.

## Order Session

The header for one ordering action that can bundle several Orders placed together, for example a blood test panel ordered in one sitting.

- Legacy alias: `ocrr_ocsorder` / `OcsOrderSession`.
- FHIR alignment: no single resource covers this cleanly - see `queries.md`.

## Specimen

The physical sample collected to satisfy one or more Orders.

- Legacy alias: `ocrr_orderspecimen` / `OrderSpecimen`, many-to-many with Order via a join table, carrying collection site, source, and a collection-status history.
- FHIR alignment: `Specimen`.

## Result / Observation

The answer to an Order: a discrete value, a narrative report, or an attached document.

- Legacy alias: `ocrr_resultdetails` / `ResultDetails`, which is polymorphic over `PathResultDetails` (structured pathology), `ClinicalResultDetail` (narrative), and `PatientDocument` (attached file) - a pattern worth keeping, since results genuinely come in those three shapes.
- FHIR alignment: `Observation` for discrete/structured results, `DiagnosticReport` for the grouping and narrative, `DocumentReference` for attachments.

## Appointment

A scheduled contact between a patient and a service or clinician.

- Legacy alias: `schl_booking_appoin` / `Booking_Appointment`, with a full status history (booked, confirmed, arrived, seen, completed, cancelled, did-not-attend) rather than a single status field.
- FHIR alignment: `Appointment`, becoming an `Encounter` once attended.

## RTT Pathway (18-week clock)

NHS England's statutory Referral-To-Treatment waiting-time pathway, tracked against every referral and its subsequent appointments and treatment. This is a UK policy requirement, not a clinical concept, and has no FHIR equivalent - it must be modelled explicitly regardless of what standard the rest of the system uses, because real NHS trusts report breaches against it.

- Legacy alias: `pthw_pathwayclock` / `PathwayClock`, `PathwaysRTTClockImpact`, `firstApptKPIDate`/`rTTBreachKPIDate` fields on `Booking_Appointment`.
- FHIR alignment: none. Model as a first-class openpas concept.

## Lookup value

A coded, centrally-maintained choice-list entry: ethnicity, marital status, specialty, and hundreds more.

- Legacy alias: `LookupInstance`, referenced from nearly every entity.
- FHIR alignment: wherever a real standard code system exists (SNOMED CT, the NHS Data Dictionary's national codes), bind to it instead of inventing a local lookup table. Reserve local lookups for genuinely local choices.
- Terms to avoid: don't call these "lookups" in user-facing copy, that's an implementation term - use the specific concept name (for example "ethnic category") in UI and reports.
