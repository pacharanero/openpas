# openpas-backend

The FHIR backend: `helios-fhir` + `helios-persistence`'s `PostgresBackend` + `helios-rest`'s `create_app_with_config`, wired together. See [`../../adr/0001-fhir-foundation.md`](../../adr/0001-fhir-foundation.md) for why.

This is the **backend** half of the backend/client split - it serves an HTTP API and owns no UI. `crates/web` is its first client; see `spec/roadmap.md`.

## Run it

```sh
docker compose up -d        # Postgres, from the repo root
cargo run -p openpas-backend
```

Listens on `127.0.0.1:8080` by default (`helios_rest::ServerConfig`'s own default). `POST /Patient`, `GET /Patient/{id}`, `GET /metadata`, and the rest of the FHIR RESTful API documented in [helios-rest's README](https://github.com/HeliosSoftware/hfs/blob/main/crates/rest/README.md) all work today.

Openpas-specific operations:

- `POST /openpas/nhs-number/validate` - NHS Number format and check-digit validation.
- `POST /openpas/patients/merge` - link-only patient merge. Accepts `survivor_id` and `duplicate_id`; atomically adds reciprocal FHIR `replaces` / `replaced-by` links and marks only the duplicate inactive. It copies no demographics or identifiers and uses FHIR version preconditions to reject concurrent changes rather than overwrite them.

## Configuration

- `OPENPAS_DB_PASSWORD` - Postgres password, defaults to `openpas-dev` to match `docker-compose.yml`. Everything else (`host`, `port`, `dbname`, `user`) uses `helios_persistence::PostgresConfig`'s own defaults (`localhost:5432/helios` as user `helios`), which `docker-compose.yml` matches.

## Test

```sh
docker compose up -d
cargo test -p openpas-backend
```

`tests/patient_fhir_api.rs` is a real end-to-end test: it builds the actual app via `build_app()` (the same function `main` calls) and exercises it through `tower::ServiceExt::oneshot`, against the real Postgres container, not a mock. It covers Patient create/read/update and the atomic link-only merge operation.

## Known dependency advisories

Two GitHub-native Dependabot alerts (not yet in RustSec's advisory-db, so `cargo audit` doesn't see them) are open against transitive dependencies whose vulnerable code paths aren't reachable in this backend's current configuration:

- **`jsonwebtoken` < 10.3.0** (GHSA-h395-gr6q-cpjc, type confusion in `nbf`/`exp` claim validation) - pulled in via `helios-auth`/`helios-rest`, which pin `jsonwebtoken = "^9"`; no 9.x patch fixes it and no newer Helios release widens the pin yet. Not reachable today: `HFS_AUTH_ENABLED` defaults to `false` and `build_app()` never sets it, so no JWT validation runs. **Before ever enabling auth**, check whether Helios has picked up `jsonwebtoken >= 10.3.0`.
- **`thrift` < 0.23.0** (GHSA-2f9f-gq7v-9h6m, excessive memory allocation) - pulled in via `parquet` (Parquet file format support in `helios-sof`'s SQL-on-FHIR feature), which pins `thrift = "^0.17"`. Mitigated by setting `ServerConfig.sof_enabled = false` in `build_app()`, since openpas doesn't use SQL-on-FHIR - see the comment there.

## What's not here yet

The broader PAS-specific domain layer from `spec/domain-model.md` - extended demographics, Episodes of Care, Admissions, Referrals, RTT pathway tracking, and the rest - is `spec/roadmap.md` Phase 1 onward.
