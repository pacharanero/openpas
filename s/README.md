# `s/` scripts

Tiny wrappers for repeated processes. Each `exec`s the underlying tool and forwards its arguments.

- `s/fmt` - `cargo fmt --all`
- `s/lint` - `cargo clippy --all-targets -- -D warnings`
- `s/test` - `cargo test` (needs a real Postgres: `docker compose up -d` first)
- `s/css` - compile the checked-in Tailwind CSS + DaisyUI stylesheet (run `npm ci --ignore-scripts` once first).
- `s/check` - Rust format/lint/test, npm audit, and reproducible CSS build, the same checks CI runs. Run before every commit.
- `s/up` - starts Postgres, the FHIR backend, and the web UI; opens the browser on the login page once it's ready. `Ctrl-C` stops both Rust processes (Postgres keeps running - `docker compose down` to stop that too).
