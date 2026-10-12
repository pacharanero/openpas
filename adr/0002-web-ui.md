# ADR-0002: Server-rendered Rust web UI (axum + Askama + htmx), session-based access control

Status: Accepted

Date: 2026-10-12

## Context

`crates/backend` is deliberately UI-less per ADR-0001 and its own `AGENTS.md` invariant. There is no client yet - what looked like a UI in `s/up` was just the raw FHIR `CapabilityStatement` JSON at `/metadata`.

The brief for the first client: a web GUI, "the primary interface initially, especially at PoC stage"; a popular framework, preferably Rust; if JS/client-side instead, access control needs solving.

A fully client-side SPA calling the FHIR API directly would need a real OAuth2/OIDC identity provider issuing JWTs the FHIR backend can verify (Helios supports this via `HFS_AUTH_*`, but it's off today - see `crates/backend/README.md`'s dismissed-advisory notes). Standing up an IdP (Keycloak or similar) is real infrastructure this project doesn't need yet for a single first-party client at PoC stage.

Researched current (October 2026) adoption rather than assuming from memory:

| Crate | Total downloads | Recent downloads | Last updated |
|---|---|---|---|
| `askama` | 53.6M | 13.9M | 2026-09-04 |
| `tower-sessions` | 4.1M | 1.07M | 2026-02-01 |
| `leptos` | 5.2M | 1.4M | 2026-10-05 |
| `dioxus` | 3.1M | 1.15M | 2026-07-31 |

Leptos and Dioxus are both popular, actively-maintained full-stack reactive frameworks - either would be a reasonable choice. Both need a WASM toolchain for hydration, which is more build-pipeline complexity than a PAS admin form (the textbook htmx use case) needs at PoC stage.

## Decision

Server-rendered Rust, not a reactive/WASM framework:

- **`axum`** (already the backend's framework, via Helios) for routing.
- **`askama`** for templates - type-checked at compile time against the template files, most-downloaded and most recently updated of the options considered.
- **`htmx`** (vendored locally, not CDN-loaded - see `static/`) for progressive interactivity: live NHS Number validation, form submission, without a SPA build pipeline. Pinned to the 2.0.11 line, not the newer 4.0.0 tag - 4.0's changelog entry wasn't fully published at research time, so 2.x's well-documented stability won: the stability is the sought property here, not newness.
- **`tower-sessions`** for session state. Using its built-in `MemoryStore` for now, not `tower-sessions-sqlx-store`: that store hasn't been updated since January 2025, which fails `dependencies.md`'s "active ownership, recent releases" bar outright. A restart loses all sessions (users log in again) - acceptable at PoC stage; revisit with a properly-maintained store, or a hand-rolled one against the same Postgres, if that stops being acceptable.

**Access control, PoC stage**: `crates/web` is a separate workspace member and a separate process (per the backend/client split in ADR-0001), the only thing with a human-facing login. It holds a session cookie per browser; the FHIR backend stays network-internal, trusted by `crates/web` with no bearer-token exchange of its own yet. This is deliberately temporary: the login itself is a single seeded demo credential, not a user-management system, and is marked as such everywhere it appears in code and docs.

**Before any of this touches real patient data or a second client appears**, revisit: proper user management, and real auth between `crates/web` and the FHIR backend (OAuth2/SMART-on-FHIR, which Helios already supports) rather than an implicitly-trusted internal network. Tracked in `spec/roadmap.md`.

## Consequences

**Easier:**

- No WASM build pipeline, no JS framework decision, no new toolchain beyond `cargo` - the whole stack stays in Rust and compiles with everything else.
- Access control is the simplest possible shape: a session cookie the server reads, nothing for client-side code to leak.
- Askama's compile-time template checking catches a typo'd field name as a build error, not a runtime blank page.

**Harder / riskier:**

- The FHIR backend's own lack of auth (deliberately dormant per ADR-0001/`crates/backend/README.md`) is now load-bearing for something real (a login-gated UI), not just "a thing we haven't configured." It stays acceptable only as long as `crates/web` is the sole client and nothing is reachable from outside this machine - revisit the moment either stops being true.
- No reactive client-side state means genuinely dynamic interactions (a live-updating RTT clock, for instance) will need more htmx polling/SSE plumbing than a reactive framework would give for free. Acceptable now; revisit if the UI's interactivity needs outgrow what htmx comfortably expresses.
- A seeded single demo credential is not user management. It must not survive contact with `spec/queries.md` query 6's "real NHS trust" ambition without being replaced.

## Alternatives considered

- **Leptos or Dioxus** (full-stack reactive, WASM hydration). Rejected for now: more toolchain and conceptual overhead than a forms-and-tables PAS admin tool needs at PoC stage. Worth revisiting if the UI's interactivity needs grow past what htmx expresses well.
- **Fully client-side SPA calling the FHIR API directly.** Rejected: needs a real OIDC provider to solve access control properly, which is infrastructure this project doesn't need yet for one first-party client.
- **In-process UI inside `crates/backend`.** Rejected: breaks the backend/client separation ADR-0001 and `AGENTS.md` already commit to.
