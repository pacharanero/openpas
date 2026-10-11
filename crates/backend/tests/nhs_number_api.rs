// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

//! End-to-end proof that NHS Number validation is actually reachable over
//! HTTP, not just unit-tested in isolation.

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn post_validate(value: &str) -> Value {
    let (app, _config) = openpas_backend::build_app()
        .await
        .expect("build_app against the Postgres backend");

    let response = app
        .oneshot(
            Request::post("/openpas/nhs-number/validate")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "value": value })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn valid_testable_number_round_trips_over_http() {
    let result = post_validate("999 100 0003").await;
    assert_eq!(result["valid"], true);
    assert_eq!(result["formatted"], "999 100 0003");
}

#[tokio::test]
async fn wrong_check_digit_is_rejected_over_http() {
    let result = post_validate("999 100 0004").await;
    assert_eq!(result["valid"], false);
    assert!(result["error"].is_string());
}

#[tokio::test]
async fn malformed_input_does_not_crash_the_endpoint() {
    let result = post_validate("definitely not an nhs number").await;
    assert_eq!(result["valid"], false);
    assert_eq!(result["formatted"], Value::Null);
}
