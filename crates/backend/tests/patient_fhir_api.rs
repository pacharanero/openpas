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
async fn creates_updates_and_reads_back_a_patient_as_real_fhir() {
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

    let updated_patient = json!({
        "resourceType": "Patient",
        "id": id,
        "identifier": [{
            "system": "https://fhir.nhs.uk/Id/nhs-number",
            "value": "9000000009"
        }],
        "name": [{
            "family": "Jones",
            "given": ["Jane"]
        }],
        "birthDate": "1985-06-15"
    });

    let update_response = app
        .clone()
        .oneshot(
            Request::put(format!("/Patient/{id}"))
                .header(http::header::CONTENT_TYPE, "application/fhir+json")
                .body(Body::from(serde_json::to_vec(&updated_patient).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

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
    assert_eq!(fetched["name"][0]["family"], "Jones");
    assert_eq!(fetched["name"][0]["given"][0], "Jane");
    assert_eq!(fetched["birthDate"], "1985-06-15");
    assert_eq!(
        fetched["identifier"][0]["system"],
        "https://fhir.nhs.uk/Id/nhs-number"
    );
}

#[tokio::test]
async fn link_only_merge_is_atomic_and_preserves_both_patients() {
    let (app, _config) = openpas_backend::build_app()
        .await
        .expect("build_app against the Postgres backend");
    let survivor_id = create_named_patient(&app, "Survivor").await;
    let duplicate_id = create_named_patient(&app, "Duplicate").await;

    let merge_response = app
        .clone()
        .oneshot(
            Request::post("/openpas/patients/merge")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "survivor_id": survivor_id,
                        "duplicate_id": duplicate_id,
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(merge_response.status(), StatusCode::OK);

    let survivor = read_patient(&app, &survivor_id).await;
    let duplicate = read_patient(&app, &duplicate_id).await;
    assert_eq!(survivor["active"], true);
    assert_eq!(survivor["name"][0]["family"], "Survivor");
    assert!(survivor["link"].as_array().unwrap().iter().any(|link| {
        link["type"] == "replaces"
            && link["other"]["reference"] == format!("Patient/{duplicate_id}")
    }));
    assert_eq!(duplicate["active"], false);
    assert_eq!(duplicate["name"][0]["family"], "Duplicate");
    assert!(duplicate["link"].as_array().unwrap().iter().any(|link| {
        link["type"] == "replaced-by"
            && link["other"]["reference"] == format!("Patient/{survivor_id}")
    }));
}

async fn create_named_patient(app: &axum::Router, family_name: &str) -> String {
    let patient = json!({
        "resourceType": "Patient",
        "name": [{"family": family_name, "given": ["Merge"]}],
        "birthDate": "1990-01-31"
    });
    let response = app
        .clone()
        .oneshot(
            Request::post("/Patient")
                .header(http::header::CONTENT_TYPE, "application/fhir+json")
                .body(Body::from(serde_json::to_vec(&patient).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let patient: Value = serde_json::from_slice(&body).unwrap();
    patient["id"].as_str().unwrap().to_string()
}

async fn read_patient(app: &axum::Router, id: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/Patient/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}
