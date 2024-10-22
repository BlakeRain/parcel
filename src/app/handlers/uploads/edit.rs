use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path},
    IntoResponse, Response,
};
use serde::Deserialize;
use serde_json::json;
use time::Date;

use crate::{
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, render_404, render_template},
    },
    env::Env,
    model::{types::Key, upload::Upload, user::hash_password},
};

#[handler]
pub async fn get_edit(
    env: Data<&Env>,
    token: &CsrfToken,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(?err, %id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return render_404("Unrecognized upload ID");
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            %user.id,
            %upload.id,
            "User tried to edit upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    render_template(
        "uploads/edit.html",
        context! {
            token => token.0,
            now => time::OffsetDateTime::now_utc(),
            has_password => upload.password.is_some(),
            upload,
            ..authorized_context(&env, &user)
        },
    )
}

#[derive(Debug, Deserialize)]
pub struct CheckSlugForm {
    token: String,
    custom_slug: String,
}

#[handler]
pub async fn post_check_slug(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
    Form(CheckSlugForm { token, custom_slug }): Form<CheckSlugForm>,
) -> poem::Result<Html<String>> {
    if !verifier.is_valid(&token) {
        tracing::error!("CSRF token is invalid in upload edit");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(?err, %id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return render_404("Unrecognized upload ID");
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            %user.id,
            %upload.id,
            "User tried to edit upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let custom_slug = custom_slug.trim().to_string();
    let exists = Upload::custom_slug_exists(&env.pool, user.id, Some(id), &custom_slug)
        .await
        .map_err(InternalServerError)?;

    render_template(
        "uploads/edit/slug.html",
        context! {
            upload,
            exists,
            custom_slug,
        },
    )
}

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");

#[derive(Debug, Deserialize)]
pub struct UploadEditForm {
    token: String,
    filename: String,
    public: Option<String>,
    limit: Option<i64>,
    #[serde(default, with = "iso8601_date::option")]
    expiry_date: Option<Date>,
    has_password: Option<String>,
    password: Option<String>,
    custom_slug: Option<String>,
}

#[handler]
pub async fn post_edit(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
    Form(UploadEditForm {
        token,
        filename,
        public,
        limit,
        expiry_date,
        has_password,
        password,
        custom_slug,
    }): Form<UploadEditForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&token) {
        tracing::error!("CSRF token is invalid in upload edit");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(mut upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(?err, %id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            %user.id,
            %upload.id,
            "User tried to edit upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let public = public.as_deref() == Some("on");

    let remaining = if upload.limit == limit {
        upload.remaining.or(limit)
    } else {
        limit
    };

    let has_password = has_password.as_deref() == Some("on");
    if has_password {
        if let Some(ref password) = password {
            upload.password = Some(hash_password(password));
        } else if upload.password.is_none() {
            tracing::error!("Password is required but not provided");
            return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
        }
    } else {
        upload.password = None;
    }

    tracing::info!(
        upload = %id,
        filename = ?filename,
        limit = ?limit,
        remaining = ?remaining,
        expiry = ?expiry_date,
        has_password = ?has_password,
        new_password = password.is_some(),
        custom_slug = ?custom_slug,
        "Updating upload");

    upload.filename = filename;
    upload.public = public;
    upload.limit = limit;
    upload.remaining = remaining;
    upload.expiry_date = expiry_date;
    upload.custom_slug = custom_slug;

    upload.save(&env.pool).await.map_err(InternalServerError)?;

    Ok(Html("")
        .with_header(
            "HX-Trigger",
            json!({
                "parcelUploadChanged": id,
            })
            .to_string(),
        )
        .into_response())
}
