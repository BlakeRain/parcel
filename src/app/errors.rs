use poem::{error::ResponseError, http::StatusCode, web::Html, IntoResponse, Response};

use super::templates::{default_context, render_template};

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
        let context = default_context();
        let Html(body) =
            render_template("errors/csrf-detected.html", &context).expect("template to render");
        Response::builder().status(self.status()).body(body)
    }
}

impl ResponseError for CsrfError {
    fn status(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}
