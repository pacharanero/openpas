# Data dictionary

Concrete field-level detail for the v1 entities in `domain-model.md`, plus a full census of the legacy modules not yet in scope.

> **Do not copy column lists from `../reference/` into this file or into any future implementation.** The tables below are an original, lean field set for a new system, not a transcription of openMAXIMS's schema. The "legacy cross-reference" section exists purely for traceability and completeness-checking - see `../reference/README.md`.

## Shared shapes

Referenced by entity tables below instead of repeating on every entity.

**Audit trail** - `created_at` (datetime), `created_by` (reference to user), `updated_at` (datetime), `updated_by` (reference to user).

**Status change** (one entry in a `status_history` list) - `from_status` (coded, nullable for the first entry), `to_status` (coded), `changed_at` (datetime), `changed_by` (reference to user), `reason` (string, nullable).

## Module 1: Patient Administration

### Patient

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| family_name, given_names | string | current name; name history kept separately |
| date_of_birth | date | |
| date_of_death | date | nullable |
| sex | coded | administrative sex; gender identity is a separate, optional field, not modelled as the same thing |
| nhs_number | string | 10 digits plus checksum |
| nhs_number_verified | boolean | separate from presence of the number |
| hospital_number | string | locally assigned |
| current_address | reference -> Address | addresses keep their own history |
| registered_gp, registered_practice | reference | stubbed for v1; GP registry detail is out of scope |
| next_of_kin | reference -> Person | optional |
| merged_into | reference -> Patient | nullable; set on merge, never hard-deleted |
| is_active | boolean | soft-delete flag |
| audit | - | see Shared shapes |

### Episode of Care

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| patient | reference -> Patient | |
| specialty | coded | current value; history in `specialty_history` |
| specialty_history | list\<Status change\>-shaped | tracks specialty changes over the episode's life, since it can change without ending the episode |
| responsible_clinician | reference | |
| opened_by_referral | reference -> Referral | |
| start_date, end_date | date | `end_date` nullable while ongoing |
| status | coded | open / closed |
| status_history | list\<Status change\> | |
| audit | - | see Shared shapes |

### Admission

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| episode_of_care | reference -> Episode of Care | |
| admission_type | coded | elective / emergency / transfer |
| start_at | datetime | |
| expected_end_at, actual_end_at | datetime | both nullable |
| location_history | list\<{ward, bed, start_at, end_at}\> | ward/bed moves during the stay, not separate entities |
| is_on_leave | boolean | |
| audit | - | see Shared shapes |

### Referral

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| patient | reference -> Patient | |
| source | coded | GP / consultant / self |
| received_at | datetime | |
| specialty_requested | coded | |
| priority | coded | |
| status | coded | received / triaged / accepted / rejected |
| status_history | list\<Status change\> | |
| audit | - | see Shared shapes |

## Module 2: Order Comms & Results

### Order Session

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| ordering_clinician | reference | |
| ordering_location | reference | |
| placed_at | datetime | |
| audit | - | see Shared shapes |

### Order (Service Request)

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| order_session | reference -> Order Session | |
| requested_item | coded | from a catalogue of tests/procedures |
| requesting_clinician | reference | |
| requested_location | reference | |
| priority | coded | |
| placer_order_number | string | assigned by openpas |
| filler_order_number | string | nullable; assigned by the fulfilling system once known |
| status | coded | requested / in_progress / resulted / cancelled |
| status_history | list\<Status change\> | |
| audit | - | see Shared shapes |

### Specimen

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| orders | list\<reference -> Order\> | many-to-many |
| collection_site | coded | for example "left arm" |
| specimen_type | coded | |
| collected_at | datetime | nullable until collected |
| received_at | datetime | nullable until received |
| status | coded | awaiting_collection / collected / received |
| status_history | list\<Status change\> | |
| audit | - | see Shared shapes |

### Result (Observation)

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| order | reference -> Order | |
| result_type | coded | structured / narrative / document |
| structured_value, unit, reference_range | decimal, string, string | populated only when `result_type = structured` |
| narrative_text | text | populated only when `result_type = narrative` |
| document | reference -> Document | populated only when `result_type = document` |
| reported_at | datetime | |
| reporting_clinician | reference | |
| review_status | coded | unreviewed / reviewed |
| audit | - | see Shared shapes |

## Module 3: Scheduling

### Session / Slot

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| service, location, clinician | reference | |
| date | date | |
| start_time, end_time | time | |
| capacity | integer | number of bookable slots |
| audit | - | see Shared shapes |

### Appointment

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| session_slot | reference -> Session / Slot | |
| patient | reference -> Patient | |
| episode_of_care or referral | reference | the clinical context the appointment serves |
| scheduled_at | datetime | |
| status | coded | booked / confirmed / arrived / seen / completed / cancelled / did_not_attend |
| status_history | list\<Status change\> | |
| outcome, outcome_reason | coded | both nullable until the appointment concludes |
| audit | - | see Shared shapes |

### RTT Pathway

| Field | Type | Notes |
|---|---|---|
| id | uuid | primary key |
| referral | reference -> Referral | |
| clock_start_date | date | |
| clock_stop_date | date | nullable until first definitive treatment |
| pauses | list\<{start_date, end_date, reason}\> | |
| breach_status | coded | |
| audit | - | see Shared shapes |

## Legacy cross-reference

Which mined legacy table(s) most closely informed each v1 entity, with column/foreign-key counts from `../reference/legacy-schema/legacy-schema.json` for traceability. Counts are legacy facts, not a target for the new schema above, which is deliberately leaner.

| v1 entity | Legacy table(s) | Columns | FKs |
|---|---|---|---|
| Patient | `core_patient` | 90 | 13 |
| Episode of Care | `core_carespell` (12 cols/2 FKs) + `core_episofcare` (16 cols/4 FKs) - openMAXIMS modelled these as two nested entities; merged here | - | - |
| Admission | `core_inpat_epis` (30 cols/2 FKs) + `core_ward_stay` (11 cols/4 FKs) + sibling `core_cons_stay` (8 cols/3 FKs) - openMAXIMS modelled ward/bed moves as a separate per-move entity; folded into one admission's location history here | - | - |
| Referral | `core_referral` | 32 | 7 |
| Order Session | `ocrr_ocsorder` | 32 | 12 |
| Order | `ocrr_orderinvestiga` | 71 | 12 |
| Specimen | `ocrr_orderspecimen` | 31 | 3 |
| Result | `ocrr_resultdetails` (+ type-specific child tables `PathResultDetails`/`ClinicalResultDetail`/`PatientDocument`, not separately counted here) | 11 | 2 |
| Session / Slot | `schl_sch_session` (41 cols/4 FKs) + `schl_sess_slot` | 24 | 5 |
| Appointment | `schl_booking_appoin` | 66 | 14 |
| RTT Pathway | `pthw_pathwayclock` | 17 | 2 |

`PathwaysRTTClockImpact` (the per-event RTT impact record referenced in `ubiquitous-language.md`) does not appear in the mined `DDL-scripts-ref147-openMAXIMS.sql` dump - noted as a gap in the mined data, not evidence that it doesn't exist in later openMAXIMS releases.

## Legacy module census (all 1,371 tables, for backlog sizing)

Generated by `../reference/scripts/extract-ddl-schema.py` from the full DDL dump. Module names are inferred from the table prefix and cross-checked against Java package names where possible; prefixes marked "unconfirmed" are a best guess from context, not verified against source. `app*` prefixes are the code-generator's own framework/platform tables (lookups, import tracking, audit, caching), not domain data.

| Prefix | Tables | Columns | FKs | Likely module | In v1 scope? |
|---|---|---|---|---|---|
| `core_` | 437 | 5850 | 833 | Core: patient, episodes, admin, clinical core, resources | Partially (Patient Admin) |
| `shcl_` | 152 | 1783 | 332 | Shared Clinical (unconfirmed) | No |
| `ther_` | 97 | 1064 | 153 | Therapies | No |
| `onco_` | 84 | 1171 | 197 | Oncology | No |
| `ns_` | 75 | 940 | 156 | Nursing | No |
| `care_` | 75 | 1114 | 205 | RefMan / Referral Management (formerly "CAREUK") | No |
| `ocrr_` | 73 | 888 | 137 | Order Comms & Results Reporting | Yes |
| `emer_` | 66 | 861 | 174 | Emergency / A&E | No |
| `med_` | 47 | 505 | 83 | Medical (unconfirmed) | No |
| `schl_` | 47 | 661 | 103 | Scheduling | Yes |
| `asse_` | 45 | 532 | 96 | Assessment | No |
| `pthw_` | 31 | 400 | 69 | Pathways / RTT | Yes (RTT Pathway only) |
| `edis_` | 27 | 401 | 64 | eDischarge | No |
| `coe_` | 25 | 222 | 17 | Care of the Elderly | No |
| `corr_` | 20 | 174 | 28 | Correspondence | No |
| `icps_` | 17 | 226 | 48 | Integrated Care Pathways (alternate variant) | No |
| `QRTZ_` | 12 | 62 | 0 | Quartz job scheduler internals | No (infrastructure) |
| `dto_` | 8 | 83 | 9 | DTO plumbing | No (infrastructure) |
| `cab_` | 8 | 72 | 6 | Choose and Book (NHS central booking integration) | No |
| `spin_` | 7 | 89 | 13 | Spinal Injuries | No |
| `adto_` | 4 | 75 | 11 | DTO plumbing variant | No (infrastructure) |
| `applookup_` | 3 | 17 | 2 | Framework lookups | No (infrastructure) |
| `hl7_` | 2 | 32 | 4 | HL7 interfacing | No |
| `appimport_`, `appentityevent_`, `appdbversion_`, `appcache_`, `appboreference_`, `appaudit_`, `apprec_`, `appreadaudit_`, `appnonaudit_` | 1 each | 1-13 each | 0 | Framework plumbing | No (infrastructure) |

Full per-table column and foreign-key detail for every row above is in [`../reference/legacy-schema/legacy-schema.json`](../reference/legacy-schema/legacy-schema.json) - consult it when a backlog module gets its turn in the roadmap, don't copy it into the spec.
