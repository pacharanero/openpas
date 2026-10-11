// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use helios_rest::ServerConfig;
use openpas_backend::build_app;

#[tokio::main]
async fn main() -> Result<()> {
    let app = build_app().await?;

    let server_config = ServerConfig::default();
    let listener = tokio::net::TcpListener::bind(server_config.socket_addr()).await?;
    println!(
        "openpas-backend listening on {}",
        server_config.socket_addr()
    );
    axum::serve(listener, app).await?;

    Ok(())
}
