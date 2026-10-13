// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use time::{Date, Month};

use crate::AppState;
use crate::repository::{NewPatient, PatientSummary};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/patients", get(list).post(create))
        .route("/patients/new", get(new_form))
        .route("/patients/new/check-nhs-number", post(check_nhs_number))
        .route("/patients/{id}", get(detail))
        .route("/patients/{id}/edit", get(edit_form).post(update))
        .route("/patients/{id}/merge", get(merge_form).post(merge_patient))
}

#[derive(Template)]
#[template(path = "patients_list.html")]
struct PatientsListTemplate {
    patients: Vec<PatientSummary>,
    merged: bool,
}

#[derive(Default, Deserialize)]
struct ListQuery {
    merged: Option<String>,
}

async fn list(State(state): State<AppState>, Query(query): Query<ListQuery>) -> Response {
    match state.repository.list_patients().await {
        Ok(patients) => Html(
            PatientsListTemplate {
                patients,
                merged: query.merged.as_deref() == Some("1"),
            }
            .render()
            .unwrap(),
        )
        .into_response(),
        Err(error) => {
            tracing_error(&error);
            internal_error_page()
        }
    }
}

#[derive(Template)]
#[template(path = "patient_new.html")]
struct PatientFormTemplate {
    page_title: String,
    heading: String,
    description: String,
    form_action: String,
    submit_label: String,
    family_name: String,
    given_name: String,
    date_of_birth: String,
    nhs_number: String,
    address_line_1: String,
    address_line_2: String,
    city: String,
    postal_code: String,
    phone: String,
    email: String,
    family_name_error: Option<String>,
    given_name_error: Option<String>,
    date_of_birth_error: Option<String>,
    nhs_number_error: Option<String>,
    address_error: Option<String>,
    email_error: Option<String>,
}

impl PatientFormTemplate {
    fn registration() -> Self {
        Self {
            page_title: "Register patient".to_string(),
            heading: "Register patient".to_string(),
            description: "Create a demographic record in the FHIR patient index.".to_string(),
            form_action: "/patients".to_string(),
            submit_label: "Register patient".to_string(),
            family_name: String::new(),
            given_name: String::new(),
            date_of_birth: String::new(),
            nhs_number: String::new(),
            address_line_1: String::new(),
            address_line_2: String::new(),
            city: String::new(),
            postal_code: String::new(),
            phone: String::new(),
            email: String::new(),
            family_name_error: None,
            given_name_error: None,
            date_of_birth_error: None,
            nhs_number_error: None,
            address_error: None,
            email_error: None,
        }
    }

    fn editing(patient: PatientSummary) -> Self {
        let mut template = Self::edit_shell(&patient.id);
        template.family_name = patient.family_name;
        template.given_name = patient.given_name;
        template.date_of_birth = patient.date_of_birth;
        template.nhs_number = patient.nhs_number.unwrap_or_default();
        if let Some(address) = patient.address {
            template.address_line_1 = address.line_1;
            template.address_line_2 = address.line_2;
            template.city = address.city;
            template.postal_code = address.postal_code;
        }
        template.phone = patient.phone.unwrap_or_default();
        template.email = patient.email.unwrap_or_default();
        template
    }

    fn edit_shell(id: &str) -> Self {
        Self {
            page_title: "Edit patient".to_string(),
            heading: "Edit patient".to_string(),
            description: "Update this patient's demographic record.".to_string(),
            form_action: format!("/patients/{id}/edit"),
            submit_label: "Save changes".to_string(),
            family_name: String::new(),
            given_name: String::new(),
            date_of_birth: String::new(),
            nhs_number: String::new(),
            address_line_1: String::new(),
            address_line_2: String::new(),
            city: String::new(),
            postal_code: String::new(),
            phone: String::new(),
            email: String::new(),
            family_name_error: None,
            given_name_error: None,
            date_of_birth_error: None,
            nhs_number_error: None,
            address_error: None,
            email_error: None,
        }
    }

    fn with_submission(
        mut self,
        form: &PatientForm,
        errors: FormErrors,
        nhs_number_error: Option<String>,
    ) -> Self {
        self.family_name.clone_from(&form.family_name);
        self.given_name.clone_from(&form.given_name);
        self.date_of_birth.clone_from(&form.date_of_birth);
        self.nhs_number.clone_from(&form.nhs_number);
        self.address_line_1.clone_from(&form.address_line_1);
        self.address_line_2.clone_from(&form.address_line_2);
        self.city.clone_from(&form.city);
        self.postal_code.clone_from(&form.postal_code);
        self.phone.clone_from(&form.phone);
        self.email.clone_from(&form.email);
        self.family_name_error = errors.family_name;
        self.given_name_error = errors.given_name;
        self.date_of_birth_error = errors.date_of_birth;
        self.address_error = errors.address;
        self.email_error = errors.email;
        self.nhs_number_error = nhs_number_error;
        self
    }
}

async fn new_form() -> Html<String> {
    Html(PatientFormTemplate::registration().render().unwrap())
}

async fn edit_form(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.repository.get_patient(&id).await {
        Ok(patient) if patient.active => {
            Html(PatientFormTemplate::editing(patient).render().unwrap()).into_response()
        }
        Ok(_) => (
            axum::http::StatusCode::CONFLICT,
            Html("<p>Merged patient records cannot be edited.</p>".to_string()),
        )
            .into_response(),
        Err(error) => {
            tracing_error(&error);
            internal_error_page()
        }
    }
}

#[derive(Template)]
#[template(path = "patient_detail.html")]
struct PatientDetailTemplate {
    patient: PatientSummary,
}

async fn detail(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.repository.get_patient(&id).await {
        Ok(patient) => Html(PatientDetailTemplate { patient }.render().unwrap()).into_response(),
        Err(error) => {
            tracing_error(&error);
            internal_error_page()
        }
    }
}

#[derive(Template)]
#[template(path = "patient_merge.html")]
struct PatientMergeTemplate {
    survivor: PatientSummary,
    candidates: Vec<PatientSummary>,
    error: Option<String>,
}

async fn merge_form(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let survivor = match state.repository.get_patient(&id).await {
        Ok(patient) if patient.active => patient,
        Ok(_) => {
            return (
                axum::http::StatusCode::CONFLICT,
                Html("<p>Merged patient records cannot be merged again.</p>".to_string()),
            )
                .into_response();
        }
        Err(error) => {
            tracing_error(&error);
            return internal_error_page();
        }
    };
    match state.repository.list_patients().await {
        Ok(patients) => {
            let candidates = patients
                .into_iter()
                .filter(|patient| patient.active && patient.id != id)
                .collect();
            Html(
                PatientMergeTemplate {
                    survivor,
                    candidates,
                    error: None,
                }
                .render()
                .unwrap(),
            )
            .into_response()
        }
        Err(error) => {
            tracing_error(&error);
            internal_error_page()
        }
    }
}

#[derive(Deserialize)]
struct MergePatientForm {
    duplicate_id: String,
    confirmed: Option<String>,
}

async fn merge_patient(
    State(state): State<AppState>,
    Path(survivor_id): Path<String>,
    Form(form): Form<MergePatientForm>,
) -> Response {
    let survivor = match state.repository.get_patient(&survivor_id).await {
        Ok(patient) => patient,
        Err(error) => {
            tracing_error(&error);
            return internal_error_page();
        }
    };
    let patients = match state.repository.list_patients().await {
        Ok(patients) => patients,
        Err(error) => {
            tracing_error(&error);
            return internal_error_page();
        }
    };
    let candidates: Vec<_> = patients
        .into_iter()
        .filter(|patient| patient.active && patient.id != survivor_id)
        .collect();
    let valid_candidate = candidates
        .iter()
        .any(|patient| patient.id == form.duplicate_id);
    if !survivor.active || !valid_candidate || form.confirmed.as_deref() != Some("yes") {
        return Html(
            PatientMergeTemplate {
                survivor,
                candidates,
                error: Some("Select an active duplicate record and confirm the merge.".to_string()),
            }
            .render()
            .unwrap(),
        )
        .into_response();
    }

    match state
        .repository
        .merge_patients(&survivor_id, &form.duplicate_id)
        .await
    {
        Ok(()) => Redirect::to("/patients?merged=1").into_response(),
        Err(error) => {
            tracing_error(&error);
            internal_error_page()
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct PatientForm {
    family_name: String,
    given_name: String,
    date_of_birth: String,
    nhs_number: String,
    address_line_1: String,
    address_line_2: String,
    city: String,
    postal_code: String,
    phone: String,
    email: String,
}

/// Pure validation, independent of the repository, so it's directly unit
/// testable without a fake or a live backend.
#[derive(Default)]
struct FormErrors {
    family_name: Option<String>,
    given_name: Option<String>,
    date_of_birth: Option<String>,
    address: Option<String>,
    email: Option<String>,
}

impl FormErrors {
    fn any(&self) -> bool {
        self.family_name.is_some()
            || self.given_name.is_some()
            || self.date_of_birth.is_some()
            || self.address.is_some()
            || self.email.is_some()
    }
}

fn validate(form: &PatientForm) -> FormErrors {
    let has_any_address = [
        &form.address_line_1,
        &form.address_line_2,
        &form.city,
        &form.postal_code,
    ]
    .iter()
    .any(|value| !value.trim().is_empty());
    FormErrors {
        family_name: form
            .family_name
            .trim()
            .is_empty()
            .then(|| "Family name is required.".to_string()),
        given_name: form
            .given_name
            .trim()
            .is_empty()
            .then(|| "Given name is required.".to_string()),
        date_of_birth: (!is_iso_date(&form.date_of_birth))
            .then(|| "Enter a valid date of birth.".to_string()),
        address: (has_any_address
            && (form.address_line_1.trim().is_empty() || form.postal_code.trim().is_empty()))
        .then(|| "Enter both the first address line and postcode.".to_string()),
        email: (!form.email.trim().is_empty() && !is_email(&form.email))
            .then(|| "Enter a valid email address.".to_string()),
    }
}

fn is_email(value: &str) -> bool {
    value
        .trim()
        .split_once('@')
        .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
}

fn is_iso_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return false;
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return false;
    }
    let Ok(year) = year.parse::<i32>() else {
        return false;
    };
    let Ok(month) = month.parse::<u8>() else {
        return false;
    };
    let Ok(month) = Month::try_from(month) else {
        return false;
    };
    let Ok(day) = day.parse::<u8>() else {
        return false;
    };
    Date::from_calendar_date(year, month, day).is_ok()
}

async fn create(State(state): State<AppState>, Form(form): Form<PatientForm>) -> Response {
    let errors = validate(&form);
    let nhs_number = form.nhs_number.trim();

    let nhs_number_error = if nhs_number.is_empty() {
        None
    } else {
        match state.repository.check_nhs_number(nhs_number).await {
            Ok(check) if check.valid => None,
            Ok(check) => Some(
                check
                    .error
                    .unwrap_or_else(|| "Not a valid NHS Number.".to_string()),
            ),
            Err(error) => {
                tracing_error(&error);
                Some("Could not validate the NHS Number right now.".to_string())
            }
        }
    };

    if errors.any() || nhs_number_error.is_some() {
        let template =
            PatientFormTemplate::registration().with_submission(&form, errors, nhs_number_error);
        return Html(template.render().unwrap()).into_response();
    }

    let input = patient_input(form);

    match state.repository.create_patient(input).await {
        Ok(_) => Redirect::to("/patients").into_response(),
        Err(error) => {
            tracing_error(&error);
            internal_error_page()
        }
    }
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<PatientForm>,
) -> Response {
    let errors = validate(&form);
    let nhs_number = form.nhs_number.trim();
    let nhs_number_error = validate_nhs_number(&state, nhs_number).await;

    if errors.any() || nhs_number_error.is_some() {
        let template =
            PatientFormTemplate::edit_shell(&id).with_submission(&form, errors, nhs_number_error);
        return Html(template.render().unwrap()).into_response();
    }

    let input = patient_input(form);

    match state.repository.update_patient(&id, input).await {
        Ok(_) => Redirect::to("/patients").into_response(),
        Err(error) => {
            tracing_error(&error);
            internal_error_page()
        }
    }
}

fn patient_input(form: PatientForm) -> NewPatient {
    NewPatient {
        family_name: form.family_name.trim().to_string(),
        given_name: form.given_name.trim().to_string(),
        date_of_birth: form.date_of_birth,
        nhs_number: optional(&form.nhs_number),
        address_line_1: optional(&form.address_line_1),
        address_line_2: optional(&form.address_line_2),
        city: optional(&form.city),
        postal_code: optional(&form.postal_code),
        phone: optional(&form.phone),
        email: optional(&form.email),
    }
}

fn optional(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

async fn validate_nhs_number(state: &AppState, nhs_number: &str) -> Option<String> {
    if nhs_number.is_empty() {
        return None;
    }
    match state.repository.check_nhs_number(nhs_number).await {
        Ok(check) if check.valid => None,
        Ok(check) => Some(
            check
                .error
                .unwrap_or_else(|| "Not a valid NHS Number.".to_string()),
        ),
        Err(error) => {
            tracing_error(&error);
            Some("Could not validate the NHS Number right now.".to_string())
        }
    }
}

#[derive(Template)]
#[template(path = "partials/nhs_number_check.html")]
struct NhsNumberCheckTemplate {
    valid: bool,
    formatted: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct CheckNhsNumberForm {
    nhs_number: String,
}

async fn check_nhs_number(
    State(state): State<AppState>,
    Form(form): Form<CheckNhsNumberForm>,
) -> Html<String> {
    if form.nhs_number.trim().is_empty() {
        return Html(String::new());
    }
    let check = match state
        .repository
        .check_nhs_number(form.nhs_number.trim())
        .await
    {
        Ok(check) => check,
        Err(error) => {
            tracing_error(&error);
            crate::repository::NhsNumberCheck {
                valid: false,
                formatted: None,
                error: Some("Could not validate the NHS Number right now.".to_string()),
            }
        }
    };
    Html(
        NhsNumberCheckTemplate {
            valid: check.valid,
            formatted: check.formatted,
            error: check.error,
        }
        .render()
        .unwrap(),
    )
}

fn tracing_error(error: &anyhow::Error) {
    eprintln!("openpas-web error: {error:#}");
}

fn internal_error_page() -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        Html("<p>Something went wrong talking to the backend. Please try again.</p>".to_string()),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_names_and_bad_dates() {
        let form = PatientForm {
            family_name: "  ".to_string(),
            given_name: "".to_string(),
            date_of_birth: "not-a-date".to_string(),
            nhs_number: String::new(),
            ..PatientForm::default()
        };
        let errors = validate(&form);
        assert!(errors.family_name.is_some());
        assert!(errors.given_name.is_some());
        assert!(errors.date_of_birth.is_some());
        assert!(errors.any());
    }

    #[test]
    fn accepts_a_well_formed_submission() {
        let form = PatientForm {
            family_name: "Smith".to_string(),
            given_name: "Jane".to_string(),
            date_of_birth: "1990-01-31".to_string(),
            nhs_number: String::new(),
            ..PatientForm::default()
        };
        let errors = validate(&form);
        assert!(!errors.any());
    }

    #[test]
    fn date_must_be_iso_shaped() {
        assert!(is_iso_date("1990-01-31"));
        assert!(is_iso_date("2000-02-29"));
        assert!(!is_iso_date("31/01/1990"));
        assert!(!is_iso_date("1990-1-31"));
        assert!(!is_iso_date("1990-02-31"));
        assert!(!is_iso_date("1900-02-29"));
        assert!(!is_iso_date(""));
    }

    #[test]
    fn rejects_partial_address_and_bad_email() {
        let form = PatientForm {
            family_name: "Smith".to_string(),
            given_name: "Jane".to_string(),
            date_of_birth: "1990-01-31".to_string(),
            city: "Leeds".to_string(),
            email: "not-an-email".to_string(),
            ..PatientForm::default()
        };
        let errors = validate(&form);
        assert!(errors.address.is_some());
        assert!(errors.email.is_some());
    }
}
