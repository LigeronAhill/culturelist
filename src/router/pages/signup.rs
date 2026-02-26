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
use tracing::instrument;
use validator::Validate;

use crate::{AppState, models::SignUpRequest, router::AuthLayer};

#[derive(Template, WebTemplate, Default)]
#[template(path = "pages/signup/page.html")]
struct SignupPage {
    title: String,
    form: SignupForm,
}
#[instrument(name = "sign up page", skip_all)]
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
    #[validate(
        length(min = 8, max = 64),
        custom(function = "validate_signup_password")
    )]
    pub confirm_password: String,
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
#[instrument(name = "sign up form", skip_all)]
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
                if e.to_string().contains("already exists") {
                    nf.email_error = Some("Почта уже зарегистрирована".into())
                } else {
                    nf.username_error = Some(e.to_string());
                }
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
#[derive(Deserialize, Debug, Serialize, Validate, Default, Clone)]
struct FormErrors<'a> {
    username_error: &'a str,
    email_error: &'a str,
    password_error: &'a str,
}
#[axum::debug_handler]
#[instrument(name = "signup form validate", skip_all)]
pub async fn signup_form_validate(
    State(state): State<Arc<AppState>>,
    ReadSignals(data): ReadSignals<SignupForm>,
) -> impl IntoResponse {
    use {
        asynk_strim::{Yielder, stream_fn},
        axum::response::{Sse, sse::Event},
        core::convert::Infallible,
        datastar::prelude::PatchSignals,
    };
    Sse::new(stream_fn(
        move |mut yielder: Yielder<Result<Event, Infallible>>| async move {
            let mut errors = FormErrors::default();
            if !data.username.is_empty() {
                let username_exists = state
                    .users_service
                    .check_username_exists(&data.username)
                    .await
                    .unwrap_or_default();
                if username_exists {
                    errors.username_error = "Имя пользователя уже занято";
                } else {
                    errors.username_error = "";
                }
                let patch = PatchSignals::new(serde_json::to_string(&errors).unwrap_or_default());
                let sse_event = patch.write_as_axum_sse_event();
                yielder.yield_item(Ok(sse_event)).await;
            }
            if let Err(err) = data.validate() {
                for (field, _) in err.errors() {
                    if field == "email" && !data.email.is_empty() {
                        errors.email_error = "Введите корректный email";
                    } else if (field == "password" && !data.password.is_empty())
                        || (field == "confirm_password" && !data.confirm_password.is_empty())
                    {
                        errors.password_error = "Требования к паролю: Заглавная буква, цифра, спецсимвол, длина от 8 до 64 символов";
                    }
                }
                let patch = PatchSignals::new(serde_json::to_string(&errors).unwrap_or_default());
                let sse_event = patch.write_as_axum_sse_event();
                yielder.yield_item(Ok(sse_event)).await;
            } else if !data.password.is_empty()
                && !data.confirm_password.is_empty()
                && data.password != data.confirm_password
            {
                errors.password_error = "Пароли не совпадают";
                let patch = PatchSignals::new(serde_json::to_string(&errors).unwrap_or_default());
                let sse_event = patch.write_as_axum_sse_event();
                yielder.yield_item(Ok(sse_event)).await;
            } else {
                let patch = PatchSignals::new(serde_json::to_string(&errors).unwrap_or_default());
                let sse_event = patch.write_as_axum_sse_event();
                yielder.yield_item(Ok(sse_event)).await;
            }
        },
    ))
}
#[instrument(name = "signup form reset", skip_all)]
pub async fn signup_form_reset() -> impl IntoResponse {
    use {
        asynk_strim::{Yielder, stream_fn},
        axum::response::{Sse, sse::Event},
        core::convert::Infallible,
        datastar::prelude::PatchSignals,
    };
    Sse::new(stream_fn(
        move |mut yielder: Yielder<Result<Event, Infallible>>| async move {
            let errors = FormErrors::default();
            let patch = PatchSignals::new(serde_json::to_string(&errors).unwrap_or_default());
            let sse_event = patch.write_as_axum_sse_event();
            yielder.yield_item(Ok(sse_event)).await;
        },
    ))
}
