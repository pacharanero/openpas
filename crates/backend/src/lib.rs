// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use helios_persistence::backends::postgres::{PostgresBackend, PostgresConfig};
use helios_rest::{ServerConfig, create_app_with_config};

/// Builds the Postgres-backed FHIR app. Shared by `main` and the integration
/// tests so both exercise the real wiring, not a stand-in.
pub async fn build_app() -> Result<axum::Router> {
    let config = PostgresConfig {
        password: Some(db_password()),
        ..Default::default()
    };

    let backend = PostgresBackend::new(config).await?;
    backend.init_schema().await?;

    Ok(create_app_with_config(backend, ServerConfig::default()))
}

fn db_password() -> String {
    std::env::var("OPENPAS_DB_PASSWORD").unwrap_or_else(|_| "openpas-dev".to_string())
}
