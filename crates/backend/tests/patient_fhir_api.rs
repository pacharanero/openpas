// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

//! End-to-end proof that the Postgres-backed FHIR substrate from
//! `adr/0001-fhir-foundation.md` actually works, not just compiles: requires a
//! real Postgres reachable per `docker-compose.yml` (`docker compose up -d`).

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn creates_and_reads_back_a_patient_as_real_fhir() {
    let (app, _config) = openpas_backend::build_app()
        .await
        .expect("build_app against the Postgres backend");

    let patient = json!({
        "resourceType": "Patient",
        "identifier": [{
            "system": "https://fhir.nhs.uk/Id/nhs-number",
            "value": "9000000009"
        }],
        "name": [{
            "family": "Smith",
            "given": ["Jane"]
        }]
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::post("/Patient")
                .header(http::header::CONTENT_TYPE, "application/fhir+json")
                .body(Body::from(serde_json::to_vec(&patient).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let created_body = create_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let created: Value = serde_json::from_slice(&created_body).unwrap();
    assert_eq!(created["resourceType"], "Patient");
    let id = created["id"]
        .as_str()
        .expect("server assigns an id on create")
        .to_string();

    let read_response = app
        .oneshot(
            Request::get(format!("/Patient/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(read_response.status(), StatusCode::OK);

    let read_body = read_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let fetched: Value = serde_json::from_slice(&read_body).unwrap();
    assert_eq!(fetched["id"], id);
    assert_eq!(fetched["name"][0]["family"], "Smith");
    assert_eq!(fetched["name"][0]["given"][0], "Jane");
    assert_eq!(
        fetched["identifier"][0]["system"],
        "https://fhir.nhs.uk/Id/nhs-number"
    );
}
