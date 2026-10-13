// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::Json;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tower::ServiceExt;

const MAX_FHIR_RESPONSE_SIZE: usize = 4 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct MergeRequest {
    pub survivor_id: String,
    pub duplicate_id: String,
}

#[derive(Debug, Serialize)]
pub struct MergeResult {
    pub survivor_id: String,
    pub duplicate_id: String,
}

type MergeError = (StatusCode, String);

pub async fn merge(
    fhir_app: axum::Router,
    Json(input): Json<MergeRequest>,
) -> Result<Json<MergeResult>, MergeError> {
    let survivor_id = input.survivor_id.trim();
    let duplicate_id = input.duplicate_id.trim();
    if survivor_id.is_empty() || duplicate_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Both survivor_id and duplicate_id are required.".to_string(),
        ));
    }
    if survivor_id == duplicate_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "A patient record cannot be merged into itself.".to_string(),
        ));
    }

    let mut survivor = read_patient(&fhir_app, survivor_id).await?;
    let mut duplicate = read_patient(&fhir_app, duplicate_id).await?;
    if survivor["active"].as_bool() == Some(false) {
        return Err((
            StatusCode::CONFLICT,
            "The surviving patient record is inactive.".to_string(),
        ));
    }
    if duplicate["active"].as_bool() == Some(false) {
        return Err((
            StatusCode::CONFLICT,
            "The duplicate patient record is already inactive.".to_string(),
        ));
    }

    let survivor_if_match = fhir_if_match(&survivor)?;
    let duplicate_if_match = fhir_if_match(&duplicate)?;

    add_link(&mut survivor, duplicate_id, "replaces");
    add_link(&mut duplicate, survivor_id, "replaced-by");
    survivor["active"] = json!(true);
    duplicate["active"] = json!(false);

    let transaction = json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": [
            {
                "resource": survivor,
                "request": {
                    "method": "PUT",
                    "url": format!("Patient/{survivor_id}"),
                    "ifMatch": survivor_if_match
                }
            },
            {
                "resource": duplicate,
                "request": {
                    "method": "PUT",
                    "url": format!("Patient/{duplicate_id}"),
                    "ifMatch": duplicate_if_match
                }
            }
        ]
    });
    let response = fhir_app
        .oneshot(
            Request::post("/")
                .header(header::CONTENT_TYPE, "application/fhir+json")
                .body(Body::from(
                    serde_json::to_vec(&transaction).map_err(internal)?,
                ))
                .map_err(internal)?,
        )
        .await
        .map_err(internal)?;
    if !response.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            "The FHIR transaction for this merge was rejected.".to_string(),
        ));
    }

    Ok(Json(MergeResult {
        survivor_id: survivor_id.to_string(),
        duplicate_id: duplicate_id.to_string(),
    }))
}

async fn read_patient(fhir_app: &axum::Router, id: &str) -> Result<Value, MergeError> {
    let response = fhir_app
        .clone()
        .oneshot(
            Request::get(format!("/Patient/{id}"))
                .body(Body::empty())
                .map_err(internal)?,
        )
        .await
        .map_err(internal)?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Patient {id} was not found."),
        ));
    }
    if !response.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            "The FHIR backend could not read a patient record.".to_string(),
        ));
    }
    let body = to_bytes(response.into_body(), MAX_FHIR_RESPONSE_SIZE)
        .await
        .map_err(internal)?;
    serde_json::from_slice(&body).map_err(internal)
}

fn add_link(patient: &mut Value, other_id: &str, link_type: &str) {
    let links = patient
        .as_object_mut()
        .expect("FHIR Patient is a JSON object")
        .entry("link")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .expect("FHIR Patient.link is an array");
    let reference = format!("Patient/{other_id}");
    if !links.iter().any(|link| {
        link["other"]["reference"].as_str() == Some(&reference)
            && link["type"].as_str() == Some(link_type)
    }) {
        links.push(json!({
            "other": {"reference": reference},
            "type": link_type
        }));
    }
}

fn fhir_if_match(patient: &Value) -> Result<String, MergeError> {
    patient["meta"]["versionId"]
        .as_str()
        .map(|version| format!("W/\"{version}\""))
        .ok_or_else(|| {
            (
                StatusCode::BAD_GATEWAY,
                "A patient record had no FHIR version for safe merging.".to_string(),
            )
        })
}

fn internal(error: impl std::fmt::Display) -> MergeError {
    eprintln!("openpas patient merge error: {error}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "The patient records could not be merged.".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_is_added_only_once() {
        let mut patient = json!({"resourceType": "Patient"});
        add_link(&mut patient, "duplicate", "replaces");
        add_link(&mut patient, "duplicate", "replaces");
        assert_eq!(patient["link"].as_array().unwrap().len(), 1);
    }
}
