// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use axum::routing::post;
use helios_persistence::backends::postgres::{PostgresBackend, PostgresConfig};
use helios_rest::{ServerConfig, create_app_with_config};

pub mod nhs_number;
pub mod patient_merge;

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
    //
    // `ServerConfig::default()` does not read `HFS_SERVER_PORT` itself - that
    // env wiring lives in ServerConfig's clap `#[arg(env = ...)]` attributes,
    // which only take effect when the struct is built by clap's parser, not
    // via `Default`. Read it ourselves so the port is still configurable.
    let server_config = ServerConfig {
        sof_enabled: false,
        port: server_port(),
        ..ServerConfig::default()
    };

    let fhir_app = create_app_with_config(backend, server_config.clone());
    let merge_fhir_app = fhir_app.clone();
    let app = fhir_app
        .route("/openpas/nhs-number/validate", post(nhs_number::validate))
        .route(
            "/openpas/patients/merge",
            post(move |input| {
                let fhir_app = merge_fhir_app.clone();
                async move { patient_merge::merge(fhir_app, input).await }
            }),
        );
    Ok((app, server_config))
}

fn db_password() -> String {
    std::env::var("OPENPAS_DB_PASSWORD").unwrap_or_else(|_| "openpas-dev".to_string())
}

fn server_port() -> u16 {
    std::env::var("HFS_SERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| ServerConfig::default().port)
}
