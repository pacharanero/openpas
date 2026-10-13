// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The seam between this UI and the FHIR backend. Deliberately plain JSON,
//! not `helios-fhir` types: this crate only ever touches a handful of
//! Patient fields, and depending on `helios-fhir` here would pull its exact
//! serde pins into a crate that otherwise doesn't need them.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct NewPatient {
    pub family_name: String,
    pub given_name: String,
    pub date_of_birth: String, // ISO 8601 date, e.g. "1990-01-31"
    pub nhs_number: Option<String>,
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostalAddress {
    pub line_1: String,
    pub line_2: String,
    pub city: String,
    pub postal_code: String,
    pub ended_on: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HistoricalContact {
    pub value: String,
    pub ended_on: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PatientSummary {
    pub id: String,
    pub family_name: String,
    pub given_name: String,
    pub date_of_birth: String,
    pub nhs_number: Option<String>,
    pub active: bool,
    pub merged_into: Option<String>,
    pub address: Option<PostalAddress>,
    pub previous_addresses: Vec<PostalAddress>,
    pub phone: Option<String>,
    pub previous_phones: Vec<HistoricalContact>,
    pub email: Option<String>,
    pub previous_emails: Vec<HistoricalContact>,
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

    pub async fn get_patient(&self, id: &str) -> Result<PatientSummary> {
        match self {
            Repository::Real(client) => client.get_patient(id).await,
            Repository::Fake(fake) => fake.get_patient(id),
        }
    }

    pub async fn update_patient(&self, id: &str, input: NewPatient) -> Result<PatientSummary> {
        match self {
            Repository::Real(client) => client.update_patient(id, input).await,
            Repository::Fake(fake) => fake.update_patient(id, input),
        }
    }

    pub async fn check_nhs_number(&self, value: &str) -> Result<NhsNumberCheck> {
        match self {
            Repository::Real(client) => client.check_nhs_number(value).await,
            Repository::Fake(fake) => Ok(fake.check_nhs_number(value)),
        }
    }

    pub async fn merge_patients(&self, survivor_id: &str, duplicate_id: &str) -> Result<()> {
        match self {
            Repository::Real(client) => client.merge_patients(survivor_id, duplicate_id).await,
            Repository::Fake(fake) => fake.merge_patients(survivor_id, duplicate_id),
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
        let body = patient_fhir_body(None, input, &today_iso());

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

    pub async fn get_patient(&self, id: &str) -> Result<PatientSummary> {
        let patient = self.get_patient_resource(id).await?;
        patient_summary_from_fhir(&patient)
    }

    async fn get_patient_resource(&self, id: &str) -> Result<Value> {
        let response = self
            .http
            .get(format!("{}/Patient/{id}", self.base_url))
            .send()
            .await
            .context("sending Patient read request to the FHIR backend")?;

        if !response.status().is_success() {
            bail!("FHIR backend rejected Patient read: {}", response.status());
        }

        response.json().await.context("parsing Patient")
    }

    pub async fn update_patient(&self, id: &str, input: NewPatient) -> Result<PatientSummary> {
        let patient = self.get_patient_resource(id).await?;
        if patient["active"].as_bool() == Some(false) {
            bail!("inactive Patient {id} cannot be edited");
        }
        let version = patient["meta"]["versionId"]
            .as_str()
            .context("FHIR Patient has no version for safe updating")?;
        let if_match = format!("W/\"{version}\"");
        let body = apply_patient_updates(patient, id, input, &today_iso());
        let response = self
            .http
            .put(format!("{}/Patient/{id}", self.base_url))
            .header("Content-Type", "application/fhir+json")
            .header("If-Match", if_match)
            .json(&body)
            .send()
            .await
            .context("sending Patient update request to the FHIR backend")?;

        if !response.status().is_success() {
            bail!(
                "FHIR backend rejected Patient update: {}",
                response.status()
            );
        }

        let updated: Value = response.json().await.context("parsing updated Patient")?;
        patient_summary_from_fhir(&updated)
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

    pub async fn merge_patients(&self, survivor_id: &str, duplicate_id: &str) -> Result<()> {
        let response = self
            .http
            .post(format!("{}/openpas/patients/merge", self.base_url))
            .json(&json!({
                "survivor_id": survivor_id,
                "duplicate_id": duplicate_id,
            }))
            .send()
            .await
            .context("sending patient merge request to the backend")?;
        if !response.status().is_success() {
            bail!("backend rejected patient merge: {}", response.status());
        }
        Ok(())
    }
}

fn patient_fhir_body(id: Option<&str>, input: NewPatient, today: &str) -> Value {
    let identifiers = input
        .nhs_number
        .as_ref()
        .map(|nhs_number| {
            vec![json!({
                "system": "https://fhir.nhs.uk/Id/nhs-number",
                "value": nhs_number,
            })]
        })
        .unwrap_or_default();
    let address = address_from_input(&input)
        .map(|address| vec![address_to_fhir(&address, "home", Some(today), None)])
        .unwrap_or_default();
    let mut telecom = Vec::new();
    if let Some(phone) = &input.phone {
        telecom.push(contact_to_fhir("phone", phone, "home", Some(today), None));
    }
    if let Some(email) = &input.email {
        telecom.push(contact_to_fhir("email", email, "home", Some(today), None));
    }
    let mut body = json!({
        "resourceType": "Patient",
        "identifier": identifiers,
        "name": [{
            "family": input.family_name,
            "given": [input.given_name],
        }],
        "birthDate": input.date_of_birth,
        "address": address,
        "telecom": telecom,
    });
    if let Some(id) = id {
        body["id"] = json!(id);
    }
    body
}

fn apply_patient_updates(mut patient: Value, id: &str, input: NewPatient, today: &str) -> Value {
    patient["id"] = json!(id);
    update_address_history(&mut patient, &input, today);
    update_contact_history(&mut patient, "phone", input.phone.as_deref(), today);
    update_contact_history(&mut patient, "email", input.email.as_deref(), today);

    let mut identifiers = patient["identifier"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    identifiers.retain(|identifier| {
        identifier["system"].as_str() != Some("https://fhir.nhs.uk/Id/nhs-number")
    });
    if let Some(nhs_number) = &input.nhs_number {
        identifiers.push(json!({
            "system": "https://fhir.nhs.uk/Id/nhs-number",
            "value": nhs_number,
        }));
    }
    patient["identifier"] = json!(identifiers);

    if patient["name"].as_array().is_none_or(Vec::is_empty) {
        patient["name"] = json!([{}]);
    }
    patient["name"][0]["family"] = json!(input.family_name);
    patient["name"][0]["given"] = json!([input.given_name]);
    patient["birthDate"] = json!(input.date_of_birth);
    patient
}

fn update_address_history(patient: &mut Value, input: &NewPatient, today: &str) {
    let replacement = address_from_input(input);
    let mut addresses = patient["address"].as_array().cloned().unwrap_or_default();
    let current_index = addresses.iter().position(is_current_home_address);
    let current = current_index.map(|index| postal_address_from_fhir(&addresses[index]));
    if current.as_ref().map(without_end_date) == replacement.as_ref().map(without_end_date) {
        return;
    }
    if let Some(index) = current_index {
        addresses[index]["use"] = json!("old");
        addresses[index]["period"]["end"] = json!(today);
    }
    if let Some(address) = replacement {
        addresses.push(address_to_fhir(&address, "home", Some(today), None));
    }
    patient["address"] = json!(addresses);
}

fn update_contact_history(
    patient: &mut Value,
    system: &str,
    replacement: Option<&str>,
    today: &str,
) {
    let mut contacts = patient["telecom"].as_array().cloned().unwrap_or_default();
    let current_index = contacts.iter().position(|contact| {
        contact["system"].as_str() == Some(system)
            && contact["use"].as_str() != Some("old")
            && contact["period"]["end"].is_null()
    });
    let current = current_index.and_then(|index| contacts[index]["value"].as_str());
    if current == replacement {
        return;
    }
    if let Some(index) = current_index {
        contacts[index]["use"] = json!("old");
        contacts[index]["period"]["end"] = json!(today);
    }
    if let Some(value) = replacement {
        contacts.push(contact_to_fhir(system, value, "home", Some(today), None));
    }
    patient["telecom"] = json!(contacts);
}

fn address_from_input(input: &NewPatient) -> Option<PostalAddress> {
    let address = PostalAddress {
        line_1: input.address_line_1.clone().unwrap_or_default(),
        line_2: input.address_line_2.clone().unwrap_or_default(),
        city: input.city.clone().unwrap_or_default(),
        postal_code: input.postal_code.clone().unwrap_or_default(),
        ended_on: None,
    };
    (![
        &address.line_1,
        &address.line_2,
        &address.city,
        &address.postal_code,
    ]
    .iter()
    .all(|value| value.is_empty()))
    .then_some(address)
}

fn address_to_fhir(
    address: &PostalAddress,
    use_code: &str,
    started_on: Option<&str>,
    ended_on: Option<&str>,
) -> Value {
    let lines: Vec<&str> = [&address.line_1, &address.line_2]
        .into_iter()
        .map(String::as_str)
        .filter(|line| !line.is_empty())
        .collect();
    let mut period = json!({});
    if let Some(start) = started_on {
        period["start"] = json!(start);
    }
    if let Some(end) = ended_on {
        period["end"] = json!(end);
    }
    json!({
        "use": use_code,
        "type": "both",
        "line": lines,
        "city": address.city,
        "postalCode": address.postal_code,
        "period": period,
    })
}

fn contact_to_fhir(
    system: &str,
    value: &str,
    use_code: &str,
    started_on: Option<&str>,
    ended_on: Option<&str>,
) -> Value {
    let mut period = json!({});
    if let Some(start) = started_on {
        period["start"] = json!(start);
    }
    if let Some(end) = ended_on {
        period["end"] = json!(end);
    }
    json!({"system": system, "value": value, "use": use_code, "period": period})
}

fn is_current_home_address(address: &Value) -> bool {
    matches!(address["use"].as_str(), None | Some("home")) && address["period"]["end"].is_null()
}

fn is_historical_address(address: &Value) -> bool {
    address["use"].as_str() == Some("old")
        || (matches!(address["use"].as_str(), None | Some("home"))
            && !address["period"]["end"].is_null())
}

fn without_end_date(address: &PostalAddress) -> PostalAddress {
    let mut address = address.clone();
    address.ended_on = None;
    address
}

fn postal_address_from_fhir(address: &Value) -> PostalAddress {
    let lines = address["line"].as_array();
    PostalAddress {
        line_1: lines
            .and_then(|lines| lines.first())
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        line_2: lines
            .and_then(|lines| lines.get(1))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        city: address["city"].as_str().unwrap_or_default().to_string(),
        postal_code: address["postalCode"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        ended_on: address["period"]["end"].as_str().map(str::to_string),
    }
}

fn today_iso() -> String {
    OffsetDateTime::now_utc().date().to_string()
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
    let nhs_number = resource["identifier"]
        .as_array()
        .and_then(|identifiers| {
            identifiers.iter().find(|identifier| {
                identifier["system"].as_str() == Some("https://fhir.nhs.uk/Id/nhs-number")
            })
        })
        .and_then(|identifier| identifier["value"].as_str())
        .map(str::to_string);
    let active = resource["active"].as_bool().unwrap_or(true);
    let merged_into = resource["link"]
        .as_array()
        .and_then(|links| {
            links
                .iter()
                .find(|link| link["type"].as_str() == Some("replaced-by"))
        })
        .and_then(|link| link["other"]["reference"].as_str())
        .and_then(|reference| reference.strip_prefix("Patient/"))
        .map(str::to_string);
    let addresses = resource["address"].as_array().cloned().unwrap_or_default();
    let address = addresses
        .iter()
        .find(|address| is_current_home_address(address))
        .map(postal_address_from_fhir);
    let previous_addresses = addresses
        .iter()
        .filter(|address| is_historical_address(address))
        .map(postal_address_from_fhir)
        .collect();
    let contacts = resource["telecom"].as_array().cloned().unwrap_or_default();
    let phone = current_contact(&contacts, "phone");
    let previous_phones = previous_contacts(&contacts, "phone");
    let email = current_contact(&contacts, "email");
    let previous_emails = previous_contacts(&contacts, "email");
    Ok(PatientSummary {
        id,
        family_name,
        given_name,
        date_of_birth,
        nhs_number,
        active,
        merged_into,
        address,
        previous_addresses,
        phone,
        previous_phones,
        email,
        previous_emails,
    })
}

fn current_contact(contacts: &[Value], system: &str) -> Option<String> {
    contacts
        .iter()
        .find(|contact| {
            contact["system"].as_str() == Some(system)
                && contact["use"].as_str() != Some("old")
                && contact["period"]["end"].is_null()
        })
        .and_then(|contact| contact["value"].as_str())
        .map(str::to_string)
}

fn previous_contacts(contacts: &[Value], system: &str) -> Vec<HistoricalContact> {
    contacts
        .iter()
        .filter(|contact| {
            contact["system"].as_str() == Some(system)
                && (contact["use"].as_str() == Some("old") || !contact["period"]["end"].is_null())
        })
        .filter_map(|contact| {
            contact["value"].as_str().map(|value| HistoricalContact {
                value: value.to_string(),
                ended_on: contact["period"]["end"].as_str().map(str::to_string),
            })
        })
        .collect()
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
        let address = address_from_input(&input);
        let summary = PatientSummary {
            id: next_id.to_string(),
            family_name: input.family_name,
            given_name: input.given_name,
            date_of_birth: input.date_of_birth,
            nhs_number: input.nhs_number,
            active: true,
            merged_into: None,
            address,
            previous_addresses: Vec::new(),
            phone: input.phone,
            previous_phones: Vec::new(),
            email: input.email,
            previous_emails: Vec::new(),
        };
        self.patients.lock().unwrap().push(summary.clone());
        Ok(summary)
    }

    fn list_patients(&self) -> Vec<PatientSummary> {
        self.patients.lock().unwrap().clone()
    }

    fn get_patient(&self, id: &str) -> Result<PatientSummary> {
        self.patients
            .lock()
            .unwrap()
            .iter()
            .find(|patient| patient.id == id)
            .cloned()
            .with_context(|| format!("patient {id} not found"))
    }

    fn update_patient(&self, id: &str, input: NewPatient) -> Result<PatientSummary> {
        let mut patients = self.patients.lock().unwrap();
        let patient = patients
            .iter_mut()
            .find(|patient| patient.id == id)
            .with_context(|| format!("patient {id} not found"))?;
        if !patient.active {
            bail!("inactive patient {id} cannot be edited");
        }
        let today = today_iso();
        let replacement_address = address_from_input(&input);
        if patient.address != replacement_address {
            if let Some(mut old_address) = patient.address.take() {
                old_address.ended_on = Some(today.clone());
                patient.previous_addresses.push(old_address);
            }
            patient.address = replacement_address;
        }
        replace_fake_contact(
            &mut patient.phone,
            &mut patient.previous_phones,
            input.phone,
            &today,
        );
        replace_fake_contact(
            &mut patient.email,
            &mut patient.previous_emails,
            input.email,
            &today,
        );
        patient.family_name = input.family_name;
        patient.given_name = input.given_name;
        patient.date_of_birth = input.date_of_birth;
        patient.nhs_number = input.nhs_number;
        Ok(patient.clone())
    }

    fn merge_patients(&self, survivor_id: &str, duplicate_id: &str) -> Result<()> {
        if survivor_id == duplicate_id {
            bail!("a patient cannot be merged into itself");
        }
        let mut patients = self.patients.lock().unwrap();
        let survivor_is_active = patients
            .iter()
            .find(|patient| patient.id == survivor_id)
            .with_context(|| format!("patient {survivor_id} not found"))?
            .active;
        if !survivor_is_active {
            bail!("surviving patient {survivor_id} is inactive");
        }
        let duplicate = patients
            .iter_mut()
            .find(|patient| patient.id == duplicate_id)
            .with_context(|| format!("patient {duplicate_id} not found"))?;
        if !duplicate.active {
            bail!("duplicate patient {duplicate_id} is inactive");
        }
        duplicate.active = false;
        duplicate.merged_into = Some(survivor_id.to_string());
        Ok(())
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

fn replace_fake_contact(
    current: &mut Option<String>,
    history: &mut Vec<HistoricalContact>,
    replacement: Option<String>,
    today: &str,
) {
    if *current == replacement {
        return;
    }
    if let Some(value) = current.take() {
        history.push(HistoricalContact {
            value,
            ended_on: Some(today.to_string()),
        });
    }
    *current = replacement;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patient_update_preserves_fields_the_ui_does_not_own() {
        let existing = json!({
            "resourceType": "Patient",
            "id": "patient-1",
            "identifier": [
                {"system": "https://fhir.nhs.uk/Id/nhs-number", "value": "9990000001"},
                {"system": "https://example.test/hospital-number", "value": "H123"}
            ],
            "name": [{"family": "Smith", "given": ["Jane"]}],
            "birthDate": "1990-01-31",
            "address": [{
                "use": "home",
                "line": ["1 Old Street"],
                "city": "Leeds",
                "postalCode": "LS1 1AA"
            }],
            "telecom": [
                {"system": "phone", "value": "01234567890", "use": "home"},
                {"system": "pager", "value": "777"}
            ]
        });

        let updated = apply_patient_updates(
            existing,
            "patient-1",
            NewPatient {
                family_name: "Jones".to_string(),
                given_name: "Jane".to_string(),
                date_of_birth: "1985-06-15".to_string(),
                nhs_number: Some("9991000003".to_string()),
                address_line_1: Some("2 New Road".to_string()),
                address_line_2: None,
                city: Some("York".to_string()),
                postal_code: Some("YO1 1AA".to_string()),
                phone: Some("01904123456".to_string()),
                email: Some("jane@example.test".to_string()),
            },
            "2026-10-13",
        );

        assert_eq!(updated["name"][0]["family"], "Jones");
        assert_eq!(updated["telecom"][0]["value"], "01234567890");
        assert_eq!(updated["telecom"][0]["use"], "old");
        assert_eq!(updated["telecom"][0]["period"]["end"], "2026-10-13");
        assert_eq!(updated["telecom"][1]["system"], "pager");
        assert_eq!(updated["telecom"][2]["value"], "01904123456");
        assert_eq!(updated["address"][0]["use"], "old");
        assert_eq!(updated["address"][0]["period"]["end"], "2026-10-13");
        assert_eq!(updated["address"][1]["line"][0], "2 New Road");
        assert!(
            updated["identifier"]
                .as_array()
                .unwrap()
                .iter()
                .any(|identifier| identifier["system"] == "https://example.test/hospital-number")
        );
    }

    #[test]
    fn patient_summary_does_not_treat_work_addresses_as_home_history() {
        let patient = json!({
            "resourceType": "Patient",
            "id": "patient-1",
            "name": [{"family": "Smith", "given": ["Jane"]}],
            "birthDate": "1990-01-31",
            "address": [
                {"use": "home", "line": ["1 Home Street"]},
                {"use": "old", "line": ["2 Old Road"], "period": {"end": "2025-01-01"}},
                {"use": "work", "line": ["3 Work Way"], "period": {"end": "2024-01-01"}}
            ]
        });

        let summary = patient_summary_from_fhir(&patient).unwrap();

        assert_eq!(summary.address.unwrap().line_1, "1 Home Street");
        assert_eq!(summary.previous_addresses.len(), 1);
        assert_eq!(summary.previous_addresses[0].line_1, "2 Old Road");
    }
}
