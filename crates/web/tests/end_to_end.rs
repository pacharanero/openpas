// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proves `openpas-web` and `openpas-backend` actually talk to each other
//! for real: spawns the real backend (which needs a real Postgres - see
//! `docker compose up -d`) as a live listener, then drives the real web app
//! against it. `tests/web_flow.rs` covers the UI's own logic against the
//! fake repository; this is the one test that checks the seam between the
//! two processes genuinely works.

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use http_body_util::BodyExt;
use openpas_web::build_app;
use openpas_web::repository::{FhirBackendClient, NewPatient, Repository};
use tower::ServiceExt;

async fn spawn_real_backend() -> String {
    let (app, _config) = openpas_backend::build_app()
        .await
        .expect("build_app against the Postgres backend");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn registers_a_patient_through_the_real_backend() {
    let backend_url = spawn_real_backend().await;
    let client = FhirBackendClient::new(backend_url);
    let web_app = build_app(Repository::Real(client.clone()));
    let created_family_name = format!("KhanE2E{}", std::process::id());

    let login_response = web_app
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

    // A valid NHS Number from the reserved testable range, checked live
    // through the real backend's /openpas/nhs-number/validate.
    let check_response = web_app
        .clone()
        .oneshot(
            Request::post("/patients/new/check-nhs-number")
                .header(http::header::COOKIE, cookie.clone())
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(Body::from("nhs_number=999 100 0003"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(check_response.status(), StatusCode::OK);
    let check_text = String::from_utf8(
        check_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(check_text.contains("Valid"));

    let create_response = web_app
        .clone()
        .oneshot(
            Request::post("/patients")
                .header(http::header::COOKIE, cookie.clone())
                .header(http::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "family_name={created_family_name}&given_name=Amira&date_of_birth=1985-06-15&nhs_number=999+100+0003&address_line_1=1+Old+Street&address_line_2=&city=Leeds&postal_code=LS1+1AA&phone=01131234567&email=amira%40old.example"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::SEE_OTHER);

    let list_response = web_app
        .clone()
        .oneshot(
            Request::get("/patients")
                .header(http::header::COOKIE, cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list_text = String::from_utf8(
        list_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(list_text.contains(&created_family_name));
    assert!(list_text.contains("Amira"));

    let patient_id = client
        .list_patients()
        .await
        .unwrap()
        .into_iter()
        .find(|patient| patient.family_name == created_family_name)
        .expect("created patient is present in the backend")
        .id;
    let updated_family_name = format!("RahmanE2E{}", std::process::id());

    let update_response = web_app
        .clone()
        .oneshot(
            Request::post(format!("/patients/{patient_id}/edit"))
                .header(http::header::COOKIE, cookie.clone())
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(Body::from(format!(
                    "family_name={updated_family_name}&given_name=Amira&date_of_birth=1985-06-15&nhs_number=999+100+0003&address_line_1=2+New+Road&address_line_2=&city=York&postal_code=YO1+1AA&phone=01904123456&email=amira%40new.example"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_response.status(), StatusCode::SEE_OTHER);

    let updated_list = web_app
        .clone()
        .oneshot(
            Request::get("/patients")
                .header(http::header::COOKIE, cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let updated_text = String::from_utf8(
        updated_list
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(updated_text.contains(&updated_family_name));
    assert!(!updated_text.contains(&created_family_name));

    let updated_patient = client.get_patient(&patient_id).await.unwrap();
    assert_eq!(
        updated_patient
            .address
            .as_ref()
            .map(|address| address.line_1.as_str()),
        Some("2 New Road")
    );
    assert!(
        updated_patient
            .previous_addresses
            .iter()
            .any(|address| address.line_1 == "1 Old Street" && address.ended_on.is_some())
    );
    assert_eq!(updated_patient.phone.as_deref(), Some("01904123456"));
    assert!(
        updated_patient
            .previous_phones
            .iter()
            .any(|phone| phone.value == "01131234567")
    );
    assert_eq!(updated_patient.email.as_deref(), Some("amira@new.example"));
    assert!(
        updated_patient
            .previous_emails
            .iter()
            .any(|email| email.value == "amira@old.example")
    );

    let duplicate = client
        .create_patient(NewPatient {
            family_name: format!("DuplicateE2E{}", std::process::id()),
            given_name: "Amira".to_string(),
            date_of_birth: "1985-06-15".to_string(),
            nhs_number: None,
            address_line_1: None,
            address_line_2: None,
            city: None,
            postal_code: None,
            phone: None,
            email: None,
        })
        .await
        .unwrap();
    let merge_response = web_app
        .oneshot(
            Request::post(format!("/patients/{patient_id}/merge"))
                .header(http::header::COOKIE, cookie)
                .header(
                    http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(Body::from(format!(
                    "duplicate_id={}&confirmed=yes",
                    duplicate.id
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(merge_response.status(), StatusCode::SEE_OTHER);

    let survivor = client.get_patient(&patient_id).await.unwrap();
    let duplicate = client.get_patient(&duplicate.id).await.unwrap();
    assert!(survivor.active);
    assert!(!duplicate.active);
    assert_eq!(duplicate.merged_into.as_deref(), Some(patient_id.as_str()));
}
