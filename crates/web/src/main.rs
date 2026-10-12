// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use openpas_web::build_app;
use openpas_web::repository::{FhirBackendClient, Repository};

#[tokio::main]
async fn main() -> Result<()> {
    let backend_url = std::env::var("OPENPAS_BACKEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8087".to_string());
    let port: u16 = std::env::var("OPENPAS_WEB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8089);

    let repository = Repository::Real(FhirBackendClient::new(backend_url.clone()));
    let app = build_app(repository);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("openpas-web listening on http://{addr} (backend: {backend_url})");
    axum::serve(listener, app).await?;

    Ok(())
}
