use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path, Query, Redirect},
};
use serde::Deserialize;
use time::Date;

use crate::{
    app::templates::{authorized_context, render_404, render_template},
    env::Env,
    model::{upload::Upload, user::User},
};

#[derive(Debug, Deserialize)]
pub struct EditQuery {
    hx_target: Option<String>,
    ult_dest: Option<String>,
}

#[handler]
pub async fn get_edit(
    env: Data<&Env>,
    token: &CsrfToken,
    user: User,
    Path(id): Path<i32>,
    Query(EditQuery {
        hx_target,
        ult_dest,
    }): Query<EditQuery>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return render_404("Unrecognized upload ID");
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            user = user.id,
            upload = upload.id,
            "User tried to edit upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    render_template(
        "uploads/edit.html",
        context! {
            token => token.0,
            now => time::OffsetDateTime::now_utc(),
            upload,
            hx_target,
            ult_dest,
            ..authorized_context(&env, &user)
        },
    )
}

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");

#[derive(Debug, Deserialize)]
pub struct UploadEditForm {
    token: String,
    ult_dest: Option<String>,
    filename: String,
    public: Option<String>,
    limit: Option<i64>,
    #[serde(default, with = "iso8601_date::option")]
    expiry_date: Option<Date>,
}

#[handler]
pub async fn post_edit(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    user: User,
    Path(id): Path<i32>,
    Form(UploadEditForm {
        token,
        ult_dest,
        filename,
        public,
        limit,
        expiry_date,
    }): Form<UploadEditForm>,
) -> poem::Result<Redirect> {
    if !verifier.is_valid(&token) {
        tracing::error!("CSRF token is invalid in upload edit");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(mut upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            user = user.id,
            upload = upload.id,
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

    tracing::info!(
        upload = id,
        filename = ?filename,
        limit = ?limit,
        remaining = ?remaining,
        expiry = ?expiry_date,
        "Updating upload");

    upload
        .edit(&env.pool, &filename, public, limit, remaining, expiry_date)
        .await
        .map_err(InternalServerError)?;

    Ok(Redirect::see_other(
        ult_dest.unwrap_or_else(|| "/uploads/list".to_string()),
    ))
}
