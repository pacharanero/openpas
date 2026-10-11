# Safety

## What this does and does not do

As of October 2026, openpas is a specification and roadmap only - see `spec/`. There is no running software, no database, and no code path that touches real patient data of any kind. Nothing in this repository can currently cause clinical harm, because nothing in it executes.

This section must be rewritten, not just reviewed, as soon as `spec/roadmap.md` Phase 1 produces running code.

## Intended users and use environment

Specification phase: developers and reviewers working on the spec itself. No clinical or administrative end users yet. Future intended users, if the project proceeds past Phase 1, are NHS (or equivalent) administrative and clinical staff using a Patient Administration System in a live care setting - which is a materially different risk profile from the current documentation-only state, and is exactly why the applicability assessment below is deliberately left open rather than pre-answered.

## Clinical safety owner

Not yet assigned. The project owner (repository owner) is the decision-maker referenced below until a dedicated Clinical Safety Officer role exists; see `spec/queries.md` query 6 for the deployment-ambition decision that determines when that needs to change.

## Current safety status

Pre-implementation. No hazard log exists yet because there is no behaviour to pose a hazard. Create `HAZARD-LOG.md` (or a `hazards/` directory, per house style `clinical-safety.md`) at or before the point `spec/roadmap.md` R7 (first real patient-registration behaviour) lands, not after.

## Medical-device applicability

This is a governance record of an assessment, not a completed assessment - the conclusion is genuinely not yet decided, and recording a placeholder here is meant to make that visible rather than paper over it.

- **Intended purpose:** not yet finalised - depends on `spec/queries.md` query 6 (real-deployment ambition vs. learning/portfolio project).
- **Decision-maker and role:** project owner (repository owner), pending a formal Clinical Safety Officer appointment.
- **Decision date:** not yet made.
- **Next review date:** before any code implementing `spec/roadmap.md` Phase 1 (R7 onward) is written, and in any case no later than the point real patient data - including synthetic data resembling it in structure and use - is used in development or testing.
- **Evidence considered:** none yet. When this is revisited, check current MHRA guidance on [borderline products](https://www.gov.uk/guidance/borderline-products-how-to-tell-if-your-product-is-a-medical-device) and [regulating medical devices in the UK](https://www.gov.uk/guidance/regulating-medical-devices-in-the-uk), and NHS England's guidance on [DCB0129 and DCB0160 applicability](https://digital.nhs.uk/services/clinical-safety/applicability-of-dcb-0129-and-dcb-0160/step-by-step-guidance), recording the access date used.
- **Conclusion:** not yet reached. A PAS that records clinical episodes, orders, and results is a strong candidate for DCB0129/DCB0160 applicability once it's real software handling real data - this note exists so that conclusion doesn't get skipped once implementation starts, not to pre-judge it.
- **Actions and owner:** revisit this file in full before Phase 1 implementation begins (see `spec/roadmap.md` R25, which explicitly depends on this).
- **Reassessment triggers:** any of - intended purpose changes, Phase 1 implementation begins, real or realistic synthetic patient data is introduced, clinical decision-support behaviour is added, or the project moves toward an actual deployment target.
