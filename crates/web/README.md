# openpas-web

The web UI: axum + Askama (templates) + htmx (progressive interactivity) + tower-sessions (login). See [`../../adr/0002-web-ui.md`](../../adr/0002-web-ui.md) for why this shape, and its access-control section before this ever touches real patient data.

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
- Patient registration (`src/patients.rs`, `templates/patient_new.html`) with live NHS Number validation over htmx (`POST /patients/new/check-nhs-number`, calling the backend's own `/openpas/nhs-number/validate`).
- A patient list (`templates/patients_list.html`) proving the round trip: register here, it's a real FHIR `Patient` on the backend, it shows up here.

## Test

```sh
cargo test -p openpas-web
```

Two tiers, matching `openpas-backend`'s pattern:

- `tests/web_flow.rs` - the UI's own logic (login, validation, redirects, error rendering) against the in-memory `FakeRepository`. No Postgres or live backend needed.
- `tests/end_to_end.rs` - spawns the real backend (needs `docker compose up -d` first) and drives the real UI against it. The one test that proves the seam between the two processes actually works, not just each side in isolation.

## What's not here yet

Only patient registration and listing - nothing from `spec/domain-model.md`'s Episode of Care, Admission, Order Comms, or Scheduling modules has a UI yet, because the backend doesn't implement that domain logic yet either (see `spec/roadmap.md`). This is deliberately the thinnest slice that proves the whole stack - browser to UI to backend to FHIR to Postgres - works end to end.
