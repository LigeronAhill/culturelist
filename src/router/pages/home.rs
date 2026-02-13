use askama::Template;
use askama_web::WebTemplate;
use axum::response::IntoResponse;

use crate::{models::User, router::AuthLayer};

#[derive(Template, WebTemplate)]
#[template(path = "pages/home/page.html")]
struct Home<'a> {
    title: &'a str,
    user: Option<User>,
}

pub async fn page(auth: AuthLayer) -> impl IntoResponse {
    let current = auth.current_user;
    Home {
        title: "КультурЛист | Главная",
        user: current,
    }
}
