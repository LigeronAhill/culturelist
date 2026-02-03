use askama::Template;
use askama_web::WebTemplate;
use axum::response::IntoResponse;

#[derive(Template, WebTemplate)]
#[template(path = "pages/home/page.html")]
struct Home<'a> {
    title: &'a str,
}

pub async fn page() -> impl IntoResponse {
    Home {
        title: "КультурЛист | Главная",
    }
}
