# `s/` scripts

Tiny wrappers for repeated processes. Each `exec`s the underlying tool and forwards its arguments.

- `s/fmt` - `cargo fmt --all`
- `s/lint` - `cargo clippy --all-targets -- -D warnings`
- `s/test` - `cargo test` (needs a real Postgres: `docker compose up -d` first)
- `s/check` - `fmt --check` + `lint` + `test`, the same checks CI runs. Run before every commit.
