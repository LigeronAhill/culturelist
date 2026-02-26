use std::sync::Arc;

use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_csrf::CsrfToken;
use datastar::axum::ReadSignals;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};
use validator::Validate;

use crate::{AppState, models::SignUpRequest, router::AuthLayer};

#[derive(Template, WebTemplate, Default)]
#[template(path = "pages/signup/page.html")]
struct SignupPage {
    title: String,
    form: SignupForm,
}

pub async fn page(auth: AuthLayer, token: CsrfToken) -> impl IntoResponse {
    if auth.current_user.is_some() {
        return Redirect::to("/").into_response();
    }
    let authenticity_token = token.authenticity_token().unwrap_or_default();
    (
        token,
        SignupPage {
            title: "Signup".to_string(),
            form: SignupForm {
                csrf_token: authenticity_token,
                ..Default::default()
            },
        },
    )
        .into_response()
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/signup/signupform.html")]
#[derive(Deserialize, Debug, Serialize, Validate, Default, Clone)]
pub struct SignupForm {
    pub username: String,
    pub username_error: Option<String>,
    #[validate(email)]
    pub email: String,
    pub email_error: Option<String>,
    #[validate(
        length(min = 8, max = 64),
        custom(function = "validate_signup_password")
    )]
    pub password: String,
    pub password_error: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub csrf_token: String,
}

fn validate_signup_password(password: &str) -> Result<(), validator::ValidationError> {
    let mut errors = Vec::new();

    if !password.chars().any(|c| c.is_uppercase()) {
        errors.push("uppercase letter required");
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        errors.push("lowercase letter required");
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        errors.push("digit required");
    }
    if !password
        .chars()
        .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c))
    {
        errors.push("special character required");
    }

    if !errors.is_empty() {
        let error_message = errors.join(", ");
        let mut error = validator::ValidationError::new("password_requirements");
        error.message = Some(format!("Password requirements not met: {error_message}").into());
        return Err(error);
    }

    Ok(())
}

#[axum::debug_handler]
pub async fn signup_form(
    auth: AuthLayer,
    token: CsrfToken,
    State(state): State<Arc<AppState>>,
    ReadSignals(form): ReadSignals<SignupForm>,
) -> impl IntoResponse {
    if token.verify(&form.csrf_token).is_err() {
        let mut nf = form.clone();
        nf.username_error = Some("wrong csrf".into());
        return nf.into_response();
    }
    if (form.email_error.as_ref().is_none()
        || form.email_error.as_ref().is_some_and(|e| e.is_empty()))
        && (form.password_error.as_ref().is_none()
            || form.password_error.as_ref().is_some_and(|e| e.is_empty()))
    {
        let csrt_token = token.authenticity_token().unwrap();
        match state
            .users_service
            .sign_up(SignUpRequest {
                username: form.username.clone(),
                email: form.email.clone(),
                password: form.password.clone(),
                first_name: form.first_name.clone(),
                last_name: form.last_name.clone(),
                bio: form.bio.clone(),
            })
            .await
        {
            Ok(res) => {
                auth.login_user(res.user.id.to_string());
                Redirect::to("/").into_response()
            }
            Err(e) => {
                let mut nf = form.clone();
                nf.username_error = Some(e.to_string());
                nf.csrf_token = csrt_token;

                nf.into_response()
            }
        }
    } else {
        let mut nf = form.clone();
        nf.csrf_token = token.authenticity_token().unwrap();
        nf.into_response()
    }
}
#[instrument(name = "signup form validate", skip(token))]
pub async fn signup_form_validate(
    token: CsrfToken,
    ReadSignals(data): ReadSignals<SignupForm>,
) -> impl IntoResponse {
    warn!("received: {data:#?}");
    match data.validate() {
        Ok(_) => {
            let mut nf = data.clone();
            nf.email_error = None;
            let password_error = if data
                .password_error
                .as_ref()
                .is_some_and(|e| e.contains("Требования"))
            {
                None
            } else {
                data.password_error
            };
            nf.bio = data.bio.as_ref().map(|b| b.trim().to_string());
            nf.password_error = password_error;
            nf.csrf_token = token.authenticity_token().unwrap_or_default();
            nf.into_response()
        }
        Err(err) => {
            let errors = err.into_errors();
            let mut email_error = None;
            let mut password_error = None;
            for (field, _) in errors {
                if field == "email" && !data.email.is_empty() {
                    email_error = Some("Введите корректный email".into())
                } else if field == "password" && !data.password.is_empty() {
                    password_error = Some("Требования к паролю: Заглавная буква, цифра, спецсимвол, длина от 8 до 64 символов".into())
                }
            }
            let mut nf = data.clone();
            nf.bio = data.bio.as_ref().map(|b| b.trim().to_string());
            nf.email_error = email_error;
            nf.password_error = password_error;
            nf.csrf_token = token.authenticity_token().unwrap_or_default();
            info!("{nf:#?}");
            nf.into_response()
        }
    }
}
