use std::ops::Deref;

use poem::{
    error::ResponseError,
    http::{header, StatusCode},
    FromRequest, Request, RequestBody,
};
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub struct Form<T>(pub T);

impl<T> Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseFormError {
    #[error("invalid method '{0}'; expected 'POST'")]
    InvalidMethod(poem::http::Method),
    #[error("expected a 'Content-Type' header with 'application/x-www-form-urlencoded'")]
    ContentTypeRequired,
    #[error("invalid content type '{0}'; expected 'application/x-www-form-urlencoded'")]
    InvalidContentType(String),
    #[error("invalid form data: {0}")]
    InvalidData(#[from] serde_html_form::de::Error),
}

impl ResponseError for ParseFormError {
    fn status(&self) -> StatusCode {
        match self {
            Self::InvalidMethod(_) => StatusCode::METHOD_NOT_ALLOWED,
            Self::InvalidContentType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::ContentTypeRequired => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::InvalidData(_) => StatusCode::BAD_REQUEST,
        }
    }
}

impl<'r, T: DeserializeOwned> FromRequest<'r> for Form<T> {
    async fn from_request(req: &'r Request, body: &mut RequestBody) -> poem::Result<Self> {
        if req.method() != "POST" {
            return Err(ParseFormError::InvalidMethod(req.method().clone()).into());
        }

        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|content_type| content_type.to_str().ok())
            .ok_or(ParseFormError::ContentTypeRequired)?;

        if !matches!(
            content_type.parse::<mime::Mime>(),
            Ok(mime)if mime.type_() == "application" &&
                (mime.subtype() == "x-www-form-urlencoded" ||
                 mime.suffix().map_or(false, |v| v == "x-www-form-urlencoded")))
        {
            return Err(ParseFormError::InvalidContentType(content_type.to_string()).into());
        }

        let body = body.take()?.into_vec().await?;
        Ok(Self(
            serde_html_form::from_bytes(&body).map_err(ParseFormError::InvalidData)?,
        ))
    }
}
