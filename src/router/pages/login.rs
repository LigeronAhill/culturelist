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
use validator::Validate;

use crate::{AppState, models::SignInRequest, router::AuthLayer};

#[derive(Template, WebTemplate, Default)]
#[template(path = "pages/login/page.html")]
struct Login {
    title: String,
    email: String,
    email_error: Option<String>,
    password: String,
    password_error: Option<String>,
    csrf_token: String,
}

pub async fn page(auth: AuthLayer, token: CsrfToken) -> impl IntoResponse {
    if auth.current_user.is_some() {
        return Redirect::to("/").into_response();
    }
    let authenticity_token = token.authenticity_token().unwrap_or_default();
    (
        token,
        Login {
            title: "Login".to_string(),
            csrf_token: authenticity_token,
            ..Default::default()
        },
    )
        .into_response()
}

#[derive(Template, WebTemplate, Deserialize, Debug, Serialize, Validate)]
#[template(path = "pages/login/loginform.html")]
pub struct LoginForm {
    #[validate(email)]
    pub email: String,
    pub email_error: Option<String>,
    #[validate(length(min = 8, max = 64), custom(function = "validate_password"))]
    pub password: String,
    pub password_error: Option<String>,
    pub csrf_token: String,
}

fn validate_password(password: &str) -> Result<(), validator::ValidationError> {
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
pub async fn login_form(
    auth: AuthLayer,
    token: CsrfToken,
    State(state): State<Arc<AppState>>,
    ReadSignals(form): ReadSignals<LoginForm>,
) -> impl IntoResponse {
    if token.verify(&form.csrf_token).is_err() {
        return LoginForm {
            email: form.email,
            email_error: Some("Invalid CSRF token".to_string()),
            password: form.password,
            password_error: None,
            csrf_token: token.authenticity_token().unwrap_or_default(),
        }
        .into_response();
    }
    if (form.email_error.as_ref().is_none()
        || form.email_error.as_ref().is_some_and(|e| e.is_empty()))
        && (form.password_error.as_ref().is_none()
            || form.password_error.as_ref().is_some_and(|e| e.is_empty()))
    {
        match state
            .users_service
            .sign_in(SignInRequest {
                email: form.email.clone(),
                password: form.password.clone(),
            })
            .await
        {
            Ok(res) => {
                auth.login_user(res.user.id.to_string());
                Redirect::to("/").into_response()
            }
            Err(e) => match e {
                crate::services::UsersServiceError::WrongCredentials(err) => LoginForm {
                    email: form.email,
                    email_error: None,
                    password: form.password,
                    password_error: Some(err),
                    csrf_token: token.authenticity_token().unwrap_or_default(),
                }
                .into_response(),
                _ => LoginForm {
                    email: form.email,
                    email_error: None,
                    password: form.password,
                    password_error: Some(e.to_string()),
                    csrf_token: token.authenticity_token().unwrap_or_default(),
                }
                .into_response(),
            },
        }
    } else {
        LoginForm {
            email: form.email,
            email_error: form.email_error,
            password: form.password,
            password_error: form.password_error,
            csrf_token: token.authenticity_token().unwrap_or_default(),
        }
        .into_response()
    }
}
pub async fn login_form_validate(
    token: CsrfToken,
    ReadSignals(data): ReadSignals<LoginForm>,
) -> impl IntoResponse {
    match data.validate() {
        Ok(_) => LoginForm {
            email: data.email,
            email_error: None,
            password: data.password,
            password_error: if data
                .password_error
                .as_ref()
                .is_some_and(|e| e.contains("Требования"))
            {
                None
            } else {
                data.password_error
            },
            csrf_token: token.authenticity_token().unwrap_or_default(),
        },
        Err(err) => {
            let errors = err.into_errors();
            let mut email_error = None;
            let mut password_error = None;
            for (field, err) in errors {
                if field == "email" {
                    if let validator::ValidationErrorsKind::Field(_) = err
                        && !data.email.is_empty()
                    {
                        email_error = Some("Введите корректный email".into())
                    }
                } else if field == "password"
                    && let validator::ValidationErrorsKind::Field(_) = err
                    && !data.password.is_empty()
                {
                    password_error = Some("Требования к паролю: Заглавная буква, цифра, спецсимвол, длина от 8 до 64 символов".into())
                }
            }
            LoginForm {
                email: data.email,
                email_error,
                password: data.password,
                password_error,
                csrf_token: token.authenticity_token().unwrap_or_default(),
            }
        }
    }
}
