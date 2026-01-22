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
        match render_template("errors/csrf-detected.html", &context).await {
            Ok(Html(body)) => Response::builder().status(self.status()).body(body),
            Err(err) => {
                tracing::error!(?err, "Failed to render CSRF error page");
                Response::builder()
                    .status(self.status())
                    .body("CSRF error detected. Please refresh and try again.")
            }
        }
    }
}

impl ResponseError for CsrfError {
    fn status(&self) -> StatusCode {
        StatusCode::FORBIDDEN
    }
}

pub async fn handle_404(_: NotFoundError) -> impl IntoResponse {
    match render_template("errors/404.html", default_context(TemplateEnv::default())).await {
        Ok(html) => html
            .with_status(StatusCode::NOT_FOUND)
            .with_header("Pragma", "no-cache")
            .with_header("Cache-Control", "no-cache, no-store, must-revalidate")
            .into_response(),
        Err(err) => {
            tracing::error!(?err, "Failed to render 404 error page");
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Page not found")
        }
    }
}

pub async fn handle_500(error: poem::Error) -> impl IntoResponse {
    tracing::error!("Internal server error: {:?}", error);

    let status = error.status();

    match render_template("errors/500.html", default_context(TemplateEnv::default())).await {
        Ok(html) => html
            .with_status(status)
            .with_header("Pragma", "no-cache")
            .with_header("Cache-Control", "no-cache, no-store, must-revalidate")
            .with_header("HX-Trigger", "closeModals")
            .into_response(),
        Err(err) => {
            tracing::error!(?err, "Failed to render 500 error page");
            Response::builder()
                .status(status)
                .body("Internal server error")
        }
    }
}
