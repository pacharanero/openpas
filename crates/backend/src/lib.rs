// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use helios_persistence::backends::postgres::{PostgresBackend, PostgresConfig};
use helios_rest::{ServerConfig, create_app_with_config};

/// Builds the Postgres-backed FHIR app. Shared by `main` and the integration
/// tests so both exercise the real wiring, not a stand-in. Returns the
/// `ServerConfig` alongside the `Router` so callers bind the same host/port
/// they configured, rather than constructing a second, possibly-divergent one.
pub async fn build_app() -> Result<(axum::Router, ServerConfig)> {
    let config = PostgresConfig {
        password: Some(db_password()),
        ..Default::default()
    };

    let backend = PostgresBackend::new(config).await?;
    backend.init_schema().await?;

    // SQL-on-FHIR's Parquet export is unused by openpas and its `parquet` ->
    // `thrift` dependency chain carries an open advisory (GHSA-2f9f-gq7v-9h6m);
    // disable the feature rather than carry the risk for something unneeded.
    let server_config = ServerConfig {
        sof_enabled: false,
        ..ServerConfig::default()
    };

    let app = create_app_with_config(backend, server_config.clone());
    Ok((app, server_config))
}

fn db_password() -> String {
    std::env::var("OPENPAS_DB_PASSWORD").unwrap_or_else(|_| "openpas-dev".to_string())
}
