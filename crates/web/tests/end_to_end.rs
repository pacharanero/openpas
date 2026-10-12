// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proves `openpas-web` and `openpas-backend` actually talk to each other
//! for real: spawns the real backend (which needs a real Postgres - see
//! `docker compose up -d`) as a live listener, then drives the real web app
//! against it. `tests/web_flow.rs` covers the UI's own logic against the
//! fake repository; this is the one test that checks the seam between the
//! two processes genuinely works.

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use http_body_util::BodyExt;
use openpas_web::build_app;
use openpas_web::repository::{FhirBackendClient, Repository};
use tower::ServiceExt;

async fn spawn_real_backend() -> String {
    let (app, _config) = openpas_backend::build_app()
        .await
        .expect("build_app against the Postgres backend");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn registers_a_patient_through_the_real_backend() {
    let backend_url = spawn_real_backend().await;
    let web_app = build_app(Repository::Real(FhirBackendClient::new(backend_url)));

    let login_response = web_app
        .clone()
        .oneshot(
            Request::post("/login")
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(Body::from("username=demo&password=openpas-demo"))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login_response
        .headers()
        .get(http::header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // A valid NHS Number from the reserved testable range, checked live
    // through the real backend's /openpas/nhs-number/validate.
    let check_response = web_app
        .clone()
        .oneshot(
            Request::post("/patients/new/check-nhs-number")
                .header(http::header::COOKIE, cookie.clone())
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(Body::from("nhs_number=999 100 0003"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(check_response.status(), StatusCode::OK);
    let check_text = String::from_utf8(
        check_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(check_text.contains("Valid"));

    let create_response = web_app
        .clone()
        .oneshot(
            Request::post("/patients")
                .header(http::header::COOKIE, cookie.clone())
                .header(http::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(
                    "family_name=Khan&given_name=Amira&date_of_birth=1985-06-15&nhs_number=999+100+0003",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::SEE_OTHER);

    let list_response = web_app
        .oneshot(
            Request::get("/patients")
                .header(http::header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list_text = String::from_utf8(
        list_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(list_text.contains("Khan"));
    assert!(list_text.contains("Amira"));
}
