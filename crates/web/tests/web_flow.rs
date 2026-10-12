// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exercises the whole UI against the in-memory `FakeRepository`, so these
//! run without Postgres or a live backend - unlike
//! `crates/backend/tests/patient_fhir_api.rs`, which deliberately does need
//! both. See `tests/end_to_end.rs` for the version that proves the two
//! processes actually talk to each other for real.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use http_body_util::BodyExt;
use openpas_web::build_app;
use openpas_web::repository::{FakeRepository, Repository};
use tower::ServiceExt;

fn app() -> axum::Router {
    build_app(Repository::Fake(Arc::new(FakeRepository::new())))
}

async fn body_text(response: axum::response::Response) -> String {
    String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap()
}

#[tokio::test]
async fn unauthenticated_requests_are_redirected_to_login() {
    let response = app()
        .oneshot(Request::get("/patients").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("location").unwrap(), "/login");
}

#[tokio::test]
async fn wrong_credentials_are_rejected() {
    let response = app()
        .oneshot(
            Request::post("/login")
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(Body::from("username=demo&password=wrong"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let text = body_text(response).await;
    assert!(text.contains("Incorrect username or password"));
}

#[tokio::test]
async fn full_login_and_patient_registration_flow() {
    let app = app();

    // 1. Log in with the demo credential and capture the session cookie.
    let login_response = app
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
    assert_eq!(login_response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        login_response.headers().get("location").unwrap(),
        "/patients"
    );
    let cookie = login_response
        .headers()
        .get(http::header::SET_COOKIE)
        .expect("login sets a session cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // 2. The patients list starts empty.
    let list_response = app
        .clone()
        .oneshot(
            Request::get("/patients")
                .header(http::header::COOKIE, cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let text = body_text(list_response).await;
    assert!(text.contains("No patients registered yet"));

    // 3. Submitting an invalid form re-renders it with errors, not a redirect.
    let invalid_response = app
        .clone()
        .oneshot(
            Request::post("/patients")
                .header(http::header::COOKIE, cookie.clone())
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(Body::from(
                    "family_name=&given_name=&date_of_birth=not-a-date&nhs_number=",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_response.status(), StatusCode::OK);
    let text = body_text(invalid_response).await;
    assert!(text.contains("Family name is required"));
    assert!(text.contains("Given name is required"));
    assert!(text.contains("Enter a valid date of birth"));

    // 4. A well-formed submission redirects to the list.
    let create_response = app
        .clone()
        .oneshot(
            Request::post("/patients")
                .header(http::header::COOKIE, cookie.clone())
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(Body::from(
                    "family_name=Smith&given_name=Jane&date_of_birth=1990-01-31&nhs_number=",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        create_response.headers().get("location").unwrap(),
        "/patients"
    );

    // 5. The new patient now shows up in the list.
    let list_response = app
        .clone()
        .oneshot(
            Request::get("/patients")
                .header(http::header::COOKIE, cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let text = body_text(list_response).await;
    assert!(text.contains("Smith"));
    assert!(text.contains("Jane"));

    // 6. Logging out clears the session, so /patients bounces to /login again.
    let logout_response = app
        .clone()
        .oneshot(
            Request::get("/logout")
                .header(http::header::COOKIE, cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout_response.status(), StatusCode::SEE_OTHER);

    let after_logout = app
        .oneshot(
            Request::get("/patients")
                .header(http::header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after_logout.status(), StatusCode::SEE_OTHER);
    assert_eq!(after_logout.headers().get("location").unwrap(), "/login");
}

#[tokio::test]
async fn nhs_number_check_endpoint_reports_format_validity() {
    let app = app();

    let login_response = app
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

    let response = app
        .oneshot(
            Request::post("/patients/new/check-nhs-number")
                .header(http::header::COOKIE, cookie)
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(Body::from("nhs_number=9999999999"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let text = body_text(response).await;
    assert!(text.contains("Valid"));
}
