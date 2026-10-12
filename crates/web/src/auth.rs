// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Session-based login. **Not user management** - a single seeded demo
//! credential, checked in-process. See ADR-0002's access-control section:
//! this is deliberately temporary, acceptable only while this UI is the
//! sole client and the FHIR backend stays network-internal.

use askama::Template;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Form, Router};
use serde::Deserialize;
use tower_sessions::Session;

const SESSION_USER_KEY: &str = "user";
const DEMO_USERNAME: &str = "demo";
const DEMO_PASSWORD: &str = "openpas-demo";

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/login", get(login_form).post(login_submit))
        .route("/logout", get(logout))
}

async fn login_form() -> Html<String> {
    Html(LoginTemplate { error: None }.render().unwrap())
}

async fn login_submit(session: Session, Form(form): Form<LoginForm>) -> Response {
    if form.username == DEMO_USERNAME && form.password == DEMO_PASSWORD {
        session
            .insert(SESSION_USER_KEY, form.username)
            .await
            .expect("session store insert");
        Redirect::to("/patients").into_response()
    } else {
        let body = LoginTemplate {
            error: Some("Incorrect username or password.".to_string()),
        }
        .render()
        .unwrap();
        (StatusCode::UNAUTHORIZED, Html(body)).into_response()
    }
}

async fn logout(session: Session) -> Redirect {
    session.flush().await.expect("session store flush");
    Redirect::to("/login")
}

/// Middleware guarding every route except `/login`. Redirects to `/login`
/// rather than answering `401`, since the only client today is a browser.
pub async fn require_session(session: Session, request: Request, next: Next) -> Response {
    let logged_in = session
        .get::<String>(SESSION_USER_KEY)
        .await
        .unwrap_or_default()
        .is_some();
    if logged_in {
        next.run(request).await
    } else {
        Redirect::to("/login").into_response()
    }
}
