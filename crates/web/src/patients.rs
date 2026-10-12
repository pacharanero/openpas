// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

use askama::Template;
use axum::extract::State;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;

use crate::AppState;
use crate::repository::{NewPatient, PatientSummary};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/patients", get(list).post(create))
        .route("/patients/new", get(new_form))
        .route("/patients/new/check-nhs-number", post(check_nhs_number))
}

#[derive(Template)]
#[template(path = "patients_list.html")]
struct PatientsListTemplate {
    patients: Vec<PatientSummary>,
}

async fn list(State(state): State<AppState>) -> Response {
    match state.repository.list_patients().await {
        Ok(patients) => Html(PatientsListTemplate { patients }.render().unwrap()).into_response(),
        Err(error) => {
            tracing_error(&error);
            internal_error_page()
        }
    }
}

#[derive(Template, Default)]
#[template(path = "patient_new.html")]
struct PatientFormTemplate {
    family_name: String,
    given_name: String,
    date_of_birth: String,
    nhs_number: String,
    family_name_error: Option<String>,
    given_name_error: Option<String>,
    date_of_birth_error: Option<String>,
    nhs_number_error: Option<String>,
}

async fn new_form() -> Html<String> {
    Html(PatientFormTemplate::default().render().unwrap())
}

#[derive(Deserialize)]
pub struct PatientForm {
    family_name: String,
    given_name: String,
    date_of_birth: String,
    nhs_number: String,
}

/// Pure validation, independent of the repository, so it's directly unit
/// testable without a fake or a live backend.
#[derive(Default)]
struct FormErrors {
    family_name: Option<String>,
    given_name: Option<String>,
    date_of_birth: Option<String>,
}

impl FormErrors {
    fn any(&self) -> bool {
        self.family_name.is_some() || self.given_name.is_some() || self.date_of_birth.is_some()
    }
}

fn validate(form: &PatientForm) -> FormErrors {
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
    }
}

fn is_iso_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    matches!(parts.as_slice(), [y, m, d]
        if y.len() == 4 && m.len() == 2 && d.len() == 2
        && y.chars().all(|c| c.is_ascii_digit())
        && m.chars().all(|c| c.is_ascii_digit())
        && d.chars().all(|c| c.is_ascii_digit()))
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
        let template = PatientFormTemplate {
            family_name: form.family_name,
            given_name: form.given_name,
            date_of_birth: form.date_of_birth,
            nhs_number: form.nhs_number,
            family_name_error: errors.family_name,
            given_name_error: errors.given_name,
            date_of_birth_error: errors.date_of_birth,
            nhs_number_error,
        };
        return Html(template.render().unwrap()).into_response();
    }

    let input = NewPatient {
        family_name: form.family_name,
        given_name: form.given_name,
        date_of_birth: form.date_of_birth,
        nhs_number: (!nhs_number.is_empty()).then(|| nhs_number.to_string()),
    };

    match state.repository.create_patient(input).await {
        Ok(_) => Redirect::to("/patients").into_response(),
        Err(error) => {
            tracing_error(&error);
            internal_error_page()
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
        };
        let errors = validate(&form);
        assert!(!errors.any());
    }

    #[test]
    fn date_must_be_iso_shaped() {
        assert!(is_iso_date("1990-01-31"));
        assert!(!is_iso_date("31/01/1990"));
        assert!(!is_iso_date("1990-1-31"));
        assert!(!is_iso_date(""));
    }
}
