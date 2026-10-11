// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

//! NHS Number validation, layered on top of the FHIR substrate rather than
//! reimplemented: the checksum algorithm, parsing, and formatting all come
//! from the [`nhs-number`](https://crates.io/crates/nhs-number) crate, which
//! backs them with property-based and adversarial-input tests this project
//! has no reason to duplicate.
//!
//! Not yet wired into `Patient` create/update - this is a standalone
//! endpoint per `spec/roadmap.md` R7, ahead of deciding how NHS Number
//! validation should actually gate Patient writes.

use std::str::FromStr;

use axum::Json;
use axum::http::StatusCode;
use nhs_number::NHSNumber;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub value: String,
}

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub formatted: Option<String>,
    pub error: Option<String>,
}

pub async fn validate(
    Json(request): Json<ValidateRequest>,
) -> (StatusCode, Json<ValidateResponse>) {
    let response = match NHSNumber::from_str(&request.value) {
        Ok(number) if number.validate_check_digit() => ValidateResponse {
            valid: true,
            formatted: Some(number.to_string()),
            error: None,
        },
        Ok(number) => ValidateResponse {
            valid: false,
            formatted: Some(number.to_string()),
            error: Some("check digit does not match the first nine digits".to_string()),
        },
        // nhs-number 0.3.0's `ParseError` carries no Display impl (that
        // arrived later upstream), so we supply our own fixed message
        // rather than echoing the rejected input - same safety principle
        // the crate's own later versions document, applied here directly.
        Err(_parse_error) => ValidateResponse {
            valid: false,
            formatted: None,
            error: Some("invalid NHS Number string".to_string()),
        },
    };
    (StatusCode::OK, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn accepts_a_valid_testable_number() {
        let Json(response) = match validate(Json(ValidateRequest {
            value: "999 100 0003".to_string(),
        }))
        .await
        {
            (StatusCode::OK, body) => body,
            other => panic!("unexpected status: {:?}", other.0),
        };
        assert!(response.valid);
        assert_eq!(response.formatted, Some("999 100 0003".to_string()));
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn rejects_a_wrong_check_digit() {
        let Json(response) = validate(Json(ValidateRequest {
            value: "999 100 0004".to_string(),
        }))
        .await
        .1;
        assert!(!response.valid);
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn rejects_malformed_input_without_echoing_it() {
        let Json(response) = validate(Json(ValidateRequest {
            value: "not a number".to_string(),
        }))
        .await
        .1;
        assert!(!response.valid);
        assert_eq!(response.formatted, None);
        let error = response.error.expect("parse error message");
        assert!(!error.contains("not a number"));
    }
}
