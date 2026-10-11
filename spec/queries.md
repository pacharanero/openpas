# Open queries

Decisions that need a human, not an agent, before implementation starts. Resolve these roughly in order - later ones depend on earlier ones.

## 1. Copyright holder and affiliation

Is openpas a personal Marcus Baw project, or affiliated with Baw Medical Ltd? House style is explicit that this should be confirmed rather than guessed for anything not obviously one or the other. Needed before any `LICENSE` file, SPDX headers, or package metadata can be written.

For now let's keep it pacharanero/Marcus Baw, but with the likely option soon of being able to transfer it to the Open Health Technology Foundation, which I am setting up soon.

## 2. Code licence

openMAXIMS is AGPLv3. Options for openpas, given the clean-room stance in `README.md`:

- **AGPLv3**, matching the spirit of the source material and guaranteeing any hosted/SaaS use stays open - the strongest copyleft, and the most likely to deter commercial adopters who don't want to open their own modifications.
- **A permissive licence (MIT/Apache-2.0)**, maximising adoption and allowing commercial NHS suppliers to build on it - but allowing a closed fork.
- **EUPL-1.2**, a copyleft licence written for and popular with European public-sector software, with explicit compatibility provisions for mixing with other licences.

No default recorded here deliberately - this is a project-shaping decision, not a documentation one.

I'm in faviour of AGPLv3 or later, as it aligns with the open-source ethos of the project and ensures that any modifications remain open and accessible to the community.

## 3. Technology stack

Not decided. House style defaults to Rust for CLI tooling, but a PAS is a full web application with a relational store, a UI, and (eventually) real-time/HL7-FHIR interfacing - a bigger, more consequential choice than the CLI default covers. Worth deciding with these in mind:

- A Rust + web framework (for example `axum`) stack keeps this consistent with the rest of Marcus Baw's projects, at the cost of a smaller pool of existing FHIR/healthcare-specific libraries and contributors to draw on.
- A JVM stack (Kotlin or Java + Spring) sits in the ecosystem with the most mature existing FHIR/HL7 tooling (HAPI FHIR, and reusable familiarity for anyone who has worked on NHS informatics before, including openMAXIMS itself).
- Something else entirely (Python, TypeScript/Node) trades off differently again.

I'm veering towards Rust since it works so well for agentic coding because of the pedantic compiler. Python and other non-strongly typed, less pedantic languages seem to allow more errors to creep into runtime.

Rust is also hugely performant and memory-safe, which is particularly valuable in a healthcare context where reliability and correctness are paramount.

## 4. Database engine

PostgreSQL is the obvious modern open-source default and the likely choice, but it hasn't been formally confirmed against the stack decision in query 3.

Yes I think PG is the right choice for the database engine.

## 5. FHIR alignment depth

`domain-model.md` assumes "align with FHIR UK Core where there's a clean match," but doesn't commit to *how*:

- Run an actual FHIR RESTful API (likely via HAPI FHIR or similar) as the system of record.
- Keep a bespoke relational schema that borrows FHIR resource and field naming for familiarity and future mapping, without being a literal FHIR server.
- A hybrid: FHIR-flavoured relational core, with a FHIR API layered on top for external interoperability only.

This is foundational enough that `roadmap.md` R5 blocks on it before Phase 1 implementation work starts.

I think go full FHIR for the system of record, ensuring maximum interoperability and alignment with existing healthcare standards. It will be a huge win to be able to say everything in the whole PAS is already FHIR-compliant.

I would cleanly separate the backend from the client in the repo code from the first commit, allowing multiple different clients to evolve all targeting the same backend - for example a web client, a mobile device client, or even command-line tools, all interacting with the same FHIR-compliant backend.

## 6. Real-world deployment ambition

Is openpas aimed at eventually being deployable in an actual NHS trust - which makes DCB0129/DCB0160 clinical safety governance and possibly MHRA medical-device obligations genuinely live concerns - or is it primarily a learning/portfolio/open-source-contribution project for now, with real deployment a distant maybe? This materially affects how much governance scaffolding belongs in `SAFETY.md` now versus later; see its "reassessment triggers" section.

Right now it's a POC, but the long-term ambition is to make it deployable in an actual NHS trust, which will necessitate adhering to DCB0129/DCB0160 clinical safety governance and potentially MHRA medical-device regulations. I can do all that stuff when the time comes.

## 7. Project name

"openpas" is the working folder name used for this specification. Confirm whether that's the intended project name going forward (and whether that name is free to use/register) or just a placeholder before any public repo, package name, or domain name gets chosen around it.

openpas is fairly available as a name (eg crates.io) so I'd go with that.

## 8. Result review and escalation

Raised during the R4 domain-model review (`domain-model.md`'s Result entity). Who actions a result, on what timescale, and what happens if the responsible clinician is unavailable? This needs secondary reviewers or escalation paths defined before `roadmap.md` R14 (result capture and review) can be built properly - right now the model only tracks `review_status` as reviewed/unreviewed, with no owner or deadline attached to the unreviewed state.

## 9. RTT compliance reporting mechanics

Raised during the R4 domain-model review (`domain-model.md`'s RTT Pathway entity). `domain-model.md` models the clock itself, but not: which metrics NHS England actually requires, how often they're reported, what precisely constitutes a breach or exception, or how the data gets exported for submission. Needs resolving before `roadmap.md` R18 (RTT clock tracking and breach reporting) goes beyond the clock mechanics - and whatever it is needs to be seamless for end users, not a manual chore.
