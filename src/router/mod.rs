use crate::{AppState, models::User, services::UsersService};
use askama::Template;
use askama_web::WebTemplate;
use axum::{
    Router,
    handler::HandlerWithoutStateExt,
    http::{Method, header},
    response::{IntoResponse, Redirect},
    routing::*,
};
use axum_csrf::{CsrfConfig, CsrfLayer, Key};
use axum_session::{SessionLayer, SessionStore};
use axum_session_auth::{AuthConfig, AuthSession, AuthSessionLayer};
use axum_session_sqlx::SessionPgPool;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{error, info_span};

mod pages;

const REQUEST_ID_HEADER: &str = "cult-request-id";

pub type AuthLayer = AuthSession<User, String, SessionPgPool, UsersService>;

pub fn init(
    allowed_origin: &str,
    session_store: SessionStore<SessionPgPool>,
    app_state: AppState,
) -> Router {
    let auth_config =
        AuthConfig::<String>::default().with_anonymous_user_id(Some(uuid::Uuid::nil().to_string()));
    let auth_layer = AuthSessionLayer::<User, String, SessionPgPool, UsersService>::new(Some(
        app_state.users_service.clone(),
    ))
    .with_config(auth_config);
    let catch_panic_layer = CatchPanicLayer::new();

    let x_request_id = axum::http::HeaderName::from_static(REQUEST_ID_HEADER);

    let request_id_middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        .layer(TraceLayer::new_for_http().make_span_with(
            |request: &axum::http::Request<axum::body::Body>| {
                let request_id = request.headers().get(REQUEST_ID_HEADER);

                match request_id {
                    Some(request_id) => info_span!(
                        "http_request",
                        request_id = ?request_id,
                    ),
                    None => {
                        error!("could not extract request_id");
                        info_span!("http_request")
                    }
                }
            },
        ))
        .layer(PropagateRequestIdLayer::new(x_request_id));

    let timeout_layer = TimeoutLayer::with_status_code(
        axum::http::StatusCode::REQUEST_TIMEOUT,
        std::time::Duration::from_secs(10),
    );
    let cors_layer = CorsLayer::new()
        .allow_origin([allowed_origin.parse().unwrap()])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::ACCEPT, header::AUTHORIZATION])
        .max_age(std::time::Duration::from_secs(60 * 60))
        .allow_credentials(true);
    let compression_layer = CompressionLayer::new();

    let cookie_key = Key::generate(); // Consider storing this in config for production
    let csrf_config = CsrfConfig::default()
        .with_key(Some(cookie_key))
        .with_cookie_name("csrf-token") // optional: customize cookie name
        .with_cookie_path("/".to_string()); // optional: customize cookie path

    let static_files_service = ServeDir::new("public")
        .append_index_html_on_directories(false)
        .precompressed_gzip()
        .precompressed_br()
        .fallback(page_not_found.into_service());

    let state = Arc::new(app_state);
    Router::new()
        .route("/", get(pages::home::page))
        .route("/signout", get(sign_out))
        .route(
            "/login",
            get(pages::login::page).post(pages::login::login_form),
        )
        .route("/login/validate", get(pages::login::login_form_validate))
        .route(
            "/signup",
            get(pages::signup::page).post(pages::signup::signup_form),
        )
        .route("/signup/validate", get(pages::signup::signup_form_validate))
        .route("/signup/reset", get(pages::signup::signup_form_reset))
        .nest_service("/public", static_files_service)
        .with_state(state)
        .layer(auth_layer)
        .layer(SessionLayer::new(session_store))
        .layer(CsrfLayer::new(csrf_config))
        .layer(TraceLayer::new_for_http())
        .layer(compression_layer)
        .layer(cors_layer)
        .layer(timeout_layer)
        .layer(request_id_middleware)
        .layer(catch_panic_layer)
        .fallback(page_not_found)
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/notfound/page.html")]
struct PageNotFound {
    uri: String,
}

async fn page_not_found(uri: axum::http::Uri) -> impl IntoResponse {
    PageNotFound {
        uri: uri.to_string(),
    }
}

async fn sign_out(auth: AuthLayer) -> impl IntoResponse {
    auth.logout_user();
    Redirect::to("/")
}
