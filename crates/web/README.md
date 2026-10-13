# openpas-web

The web UI: axum + Askama (templates) + htmx (progressive interactivity) + tower-sessions (login), with DaisyUI/Tailwind CSS for its responsive dashboard shell and components. See [`../../adr/0002-web-ui.md`](../../adr/0002-web-ui.md) for why this shape, and its access-control section before this ever touches real patient data.

This is the **client** half of the backend/client split from ADR-0001 - a separate process, talking to `openpas-backend` over HTTP like any other FHIR client would, not sharing in-process state with it.

## Run it

```sh
s/up
```

brings up Postgres, the backend, and this UI together, and opens the browser on the login page once it's ready. To run just this crate against an already-running backend:

```sh
OPENPAS_BACKEND_URL=http://127.0.0.1:8087 cargo run -p openpas-web
```

Listens on `127.0.0.1:8089` by default (`OPENPAS_WEB_PORT` to change it).

## Log in

A single seeded demo credential, **not user management** - see `src/auth.rs` and ADR-0002:

- Username: `demo`
- Password: `openpas-demo`

## What's here

- Login / logout (`src/auth.rs`), session-gated via `tower_sessions`.
- Patient registration and editing (`src/patients.rs`, `templates/patient_new.html`) with live NHS Number validation over htmx (`POST /patients/new/check-nhs-number`, calling the backend's own `/openpas/nhs-number/validate`).
- A patient list (`templates/patients_list.html`) proving the round trip: register or update here, it is persisted as a real FHIR `Patient` on the backend, and the changed record shows up here.
- Confirmed link-only patient merge (`templates/patient_merge.html`): explicitly choose the surviving record and duplicate, then call the backend's atomic merge operation. The duplicate stays visible but inactive; no demographics or identifiers move automatically.
- Optional home address, phone, and email with FHIR-native history. Editing retains prior values with an end date; `templates/patient_detail.html` shows current and historical contact details.

## Build the CSS

The generated `static/style.css` is checked in so running the Rust service does not require Node. Rebuild it after changing templates or `static/input.css`:

```sh
npm ci --ignore-scripts
s/css
```

DaisyUI is at the current `5.7.47`. Tailwind and its CLI are pinned together at `4.3.0`: the newer CLI line currently hard-pins `@parcel/watcher 2.5.1`, which pulls the high-severity `GHSA-vfj7-8cjw-p6xm`; `4.3.0` permits the fixed watcher `2.6.0`. `npm audit` is part of `s/check` and CI.

## Test

```sh
cargo test -p openpas-web
```

Two tiers, matching `openpas-backend`'s pattern:

- `tests/web_flow.rs` - the UI's own logic (login, validation, redirects, error rendering) against the in-memory `FakeRepository`. No Postgres or live backend needed.
- `tests/end_to_end.rs` - spawns the real backend (needs `docker compose up -d` first) and drives the real UI against it. The one test that proves the seam between the two processes actually works, not just each side in isolation.

## What's not here yet

Name history and other extended demographics, plus everything from `spec/domain-model.md`'s Episode of Care, Admission, Referral, Order Comms, and Scheduling modules, remain unimplemented (see `spec/roadmap.md`). Merge reversal also needs a separately specified, audited workflow before implementation.
