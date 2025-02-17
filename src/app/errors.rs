use poem::{
    error::{NotFoundError, ResponseError},
    http::StatusCode,
    web::Html,
    IntoResponse, Response,
};

use super::templates::{default_context, render_template, TemplateEnv};

#[derive(Debug, thiserror::Error)]
#[error("Not signed in")]
pub struct NotSignedInError;

impl NotSignedInError {
    pub async fn handle(self) -> impl IntoResponse {
        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header("location", "/user/signin")
            .body("You need to <a href=\"/user/signin\">sign in</a>")
    }
}

impl ResponseError for NotSignedInError {
    fn status(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}

#[derive(Debug, thiserror::Error)]
#[error("CSRF detected")]
pub struct CsrfError;

impl CsrfError {
    pub async fn handle(self) -> impl IntoResponse {
        let context = default_context(TemplateEnv::default());
        let Html(body) = render_template("errors/csrf-detected.html", &context)
            .await
            .expect("template to render");
        Response::builder().status(self.status()).body(body)
    }
}

impl ResponseError for CsrfError {
    fn status(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}

pub async fn handle_404(_: NotFoundError) -> impl IntoResponse {
    render_template("errors/404.html", default_context(TemplateEnv::default()))
        .await
        .expect("failed to render 404 page")
        .with_status(StatusCode::NOT_FOUND)
        .with_header("Pragma", "no-cache")
        .with_header("Cache-Control", "no-cache, no-store, must-revalidate")
        .into_response()
}

pub async fn handle_500(error: poem::Error) -> impl IntoResponse {
    tracing::error!("Internal server error: {:?}", error);

    render_template(
        "errors/500.html",
        minijinja::context! {
            error => error.to_string(),
            ..default_context(TemplateEnv::default())
        },
    )
    .await
    .expect("failed to render 500 page")
    .with_status(error.status())
    .with_header("Pragma", "no-cache")
    .with_header("Cache-Control", "no-cache, no-store, must-revalidate")
    .into_response()
}
