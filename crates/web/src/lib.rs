// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod auth;
pub mod patients;
pub mod repository;

use axum::Router;
use axum::middleware;
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use repository::Repository;

#[derive(Clone)]
pub struct AppState {
    pub repository: Repository,
}

/// Builds the UI app for a given repository (real or fake). Shared by
/// `main` and tests, same pattern as `openpas-backend`'s `build_app`.
pub fn build_app(repository: Repository) -> Router {
    let state = AppState { repository };
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let protected = patients::router()
        .route_layer(middleware::from_fn(auth::require_session))
        .route(
            "/",
            axum::routing::get(|| async { axum::response::Redirect::to("/patients") }),
        );

    Router::new()
        .merge(auth::router())
        .merge(protected)
        .nest_service("/static", ServeDir::new(static_dir()))
        .with_state(state)
        .layer(session_layer)
}

fn static_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static")
}
