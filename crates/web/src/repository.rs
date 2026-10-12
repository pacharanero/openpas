// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The seam between this UI and the FHIR backend. Deliberately plain JSON,
//! not `helios-fhir` types: this crate only ever touches a handful of
//! Patient fields, and depending on `helios-fhir` here would pull its exact
//! serde pins into a crate that otherwise doesn't need them.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct NewPatient {
    pub family_name: String,
    pub given_name: String,
    pub date_of_birth: String, // ISO 8601 date, e.g. "1990-01-31"
    pub nhs_number: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PatientSummary {
    pub id: String,
    pub family_name: String,
    pub given_name: String,
    pub date_of_birth: String,
}

#[derive(Debug, Clone)]
pub struct NhsNumberCheck {
    pub valid: bool,
    pub formatted: Option<String>,
    pub error: Option<String>,
}

/// Either a real HTTP client against the FHIR backend, or an in-memory fake
/// for tests that should not need a live backend or Postgres. An enum, not
/// `dyn Trait`, keeps this dependency-free (no `async-trait`) for two
/// variants.
#[derive(Clone)]
pub enum Repository {
    Real(FhirBackendClient),
    Fake(Arc<FakeRepository>),
}

impl Repository {
    pub async fn create_patient(&self, input: NewPatient) -> Result<PatientSummary> {
        match self {
            Repository::Real(client) => client.create_patient(input).await,
            Repository::Fake(fake) => fake.create_patient(input),
        }
    }

    pub async fn list_patients(&self) -> Result<Vec<PatientSummary>> {
        match self {
            Repository::Real(client) => client.list_patients().await,
            Repository::Fake(fake) => Ok(fake.list_patients()),
        }
    }

    pub async fn check_nhs_number(&self, value: &str) -> Result<NhsNumberCheck> {
        match self {
            Repository::Real(client) => client.check_nhs_number(value).await,
            Repository::Fake(fake) => Ok(fake.check_nhs_number(value)),
        }
    }
}

#[derive(Clone)]
pub struct FhirBackendClient {
    http: reqwest::Client,
    base_url: String,
}

impl FhirBackendClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    pub async fn create_patient(&self, input: NewPatient) -> Result<PatientSummary> {
        let mut identifiers = Vec::new();
        if let Some(nhs_number) = &input.nhs_number {
            identifiers.push(json!({
                "system": "https://fhir.nhs.uk/Id/nhs-number",
                "value": nhs_number,
            }));
        }
        let body = json!({
            "resourceType": "Patient",
            "identifier": identifiers,
            "name": [{
                "family": input.family_name,
                "given": [input.given_name],
            }],
            "birthDate": input.date_of_birth,
        });

        let response = self
            .http
            .post(format!("{}/Patient", self.base_url))
            .header("Content-Type", "application/fhir+json")
            .json(&body)
            .send()
            .await
            .context("sending Patient create request to the FHIR backend")?;

        if !response.status().is_success() {
            bail!(
                "FHIR backend rejected Patient create: {}",
                response.status()
            );
        }

        let created: Value = response.json().await.context("parsing created Patient")?;
        patient_summary_from_fhir(&created)
    }

    pub async fn list_patients(&self) -> Result<Vec<PatientSummary>> {
        let response = self
            .http
            .get(format!("{}/Patient", self.base_url))
            .send()
            .await
            .context("sending Patient search request to the FHIR backend")?;

        if !response.status().is_success() {
            bail!(
                "FHIR backend rejected Patient search: {}",
                response.status()
            );
        }

        let bundle: Value = response.json().await.context("parsing Patient Bundle")?;
        let entries = bundle["entry"].as_array().cloned().unwrap_or_default();
        entries
            .iter()
            .map(|entry| patient_summary_from_fhir(&entry["resource"]))
            .collect()
    }

    pub async fn check_nhs_number(&self, value: &str) -> Result<NhsNumberCheck> {
        let response = self
            .http
            .post(format!("{}/openpas/nhs-number/validate", self.base_url))
            .json(&json!({ "value": value }))
            .send()
            .await
            .context("sending NHS Number validation request to the FHIR backend")?;

        let body: Value = response
            .json()
            .await
            .context("parsing validation response")?;
        Ok(NhsNumberCheck {
            valid: body["valid"].as_bool().unwrap_or(false),
            formatted: body["formatted"].as_str().map(str::to_string),
            error: body["error"].as_str().map(str::to_string),
        })
    }
}

fn patient_summary_from_fhir(resource: &Value) -> Result<PatientSummary> {
    let id = resource["id"]
        .as_str()
        .context("FHIR Patient has no id")?
        .to_string();
    let name = &resource["name"][0];
    let family_name = name["family"].as_str().unwrap_or_default().to_string();
    let given_name = name["given"][0].as_str().unwrap_or_default().to_string();
    let date_of_birth = resource["birthDate"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    Ok(PatientSummary {
        id,
        family_name,
        given_name,
        date_of_birth,
    })
}

/// In-memory stand-in for tests. `check_nhs_number` is a deliberately crude
/// format check, not the real Modulus 11 checksum - that algorithm is
/// already thoroughly tested in `crates/backend` via the real `nhs-number`
/// crate, and duplicating it here would mean pulling that dependency (and
/// its 0.3.0 pin, see `crates/backend/Cargo.toml`) into a crate that
/// otherwise doesn't need it, just to re-test something already tested.
#[derive(Default)]
pub struct FakeRepository {
    patients: Mutex<Vec<PatientSummary>>,
    next_id: Mutex<u64>,
}

impl FakeRepository {
    pub fn new() -> Self {
        Self::default()
    }

    fn create_patient(&self, input: NewPatient) -> Result<PatientSummary> {
        let mut next_id = self.next_id.lock().unwrap();
        *next_id += 1;
        let summary = PatientSummary {
            id: next_id.to_string(),
            family_name: input.family_name,
            given_name: input.given_name,
            date_of_birth: input.date_of_birth,
        };
        self.patients.lock().unwrap().push(summary.clone());
        Ok(summary)
    }

    fn list_patients(&self) -> Vec<PatientSummary> {
        self.patients.lock().unwrap().clone()
    }

    fn check_nhs_number(&self, value: &str) -> NhsNumberCheck {
        let digits: String = value.chars().filter(|c| !c.is_whitespace()).collect();
        if digits.len() == 10 && digits.chars().all(|c| c.is_ascii_digit()) {
            NhsNumberCheck {
                valid: true,
                formatted: Some(value.to_string()),
                error: None,
            }
        } else {
            NhsNumberCheck {
                valid: false,
                formatted: None,
                error: Some("invalid NHS Number string".to_string()),
            }
        }
    }
}
