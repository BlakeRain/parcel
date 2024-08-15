use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    session::Session,
    web::{CsrfToken, Data, Html, Path, Query},
    IntoResponse,
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    app::templates::{authorized_context, default_context, render_template},
    env::Env,
    model::{upload::Upload, user::User},
    utils::SessionExt,
};

async fn render_upload(
    env: Data<&Env>,
    user: Option<User>,
    session: &Session,
    csrf_token: &CsrfToken,
    upload: Upload,
) -> poem::Result<Html<String>> {
    let owner = if let Some(user) = &user {
        user.admin || upload.uploaded_by == user.id
    } else {
        false
    };

    if !upload.public && !owner {
        tracing::error!(
            user = ?user,
            upload = ?upload,
            "User tried to access private upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    }

    let Some(uploader) = User::get(&env.pool, upload.uploaded_by)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, user_id = ?upload.uploaded_by,
                            "Unable to get user by ID");
            InternalServerError(err)
        })?
    else {
        tracing::error!(user_id = ?upload.uploaded_by, "Unable to find user with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let exhausted = if let Some(remaining) = upload.remaining {
        remaining < 1
    } else {
        false
    };

    let expired = if let Some(expiry) = upload.expiry_date {
        expiry < OffsetDateTime::now_utc().date()
    } else {
        false
    };

    let can_download = !exhausted && !expired;

    render_template(
        "uploads/view.html",
        context! {
            exhausted,
            expired,
            upload,
            uploader,
            owner,
            can_download,
            has_password => upload.password.is_some(),
            csrf_token => csrf_token.0,
            error => session.take::<String>("download_error"),
            ..if let Some(user) = &user {
                authorized_context(&env, user)
            } else {
                default_context(&env)
            }
        },
    )
}

#[handler]
pub async fn get_upload(
    env: Data<&Env>,
    session: &Session,
    user: Option<User>,
    csrf_token: &CsrfToken,
    Path(slug): Path<String>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get_by_slug(&env.pool, &slug).await.map_err(|err| {
        tracing::error!(err = ?err, slug = ?slug, "Unable to get upload by slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(slug = ?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    render_upload(env, user, session, csrf_token, upload).await
}

#[handler]
pub async fn get_custom_upload(
    env: Data<&Env>,
    session: &Session,
    user: Option<User>,
    csrf_token: &CsrfToken,
    Path((owner, slug)): Path<(String, String)>,
) -> poem::Result<Html<String>> {
    let Some(owner) = User::get_by_username(&env.pool, &owner)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, owner = ?owner, "Unable to get user by username");
            InternalServerError(err)
        })?
    else {
        tracing::error!(owner = ?owner, "Unable to find user with given username");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let Some(upload) = Upload::get_by_custom_slug(&env.pool, owner.id, &slug)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, slug = ?slug, "Unable to get upload by slug");
            InternalServerError(err)
        })?
    else {
        tracing::error!(slug = ?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    render_upload(env, user, session, csrf_token, upload).await
}

#[handler]
pub async fn delete_upload(
    env: Data<&Env>,
    user: User,
    Path(id): Path<i32>,
) -> poem::Result<poem::Response> {
    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!(id = ?id, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            user = user.id,
            upload = upload.id,
            "User tried to delete upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    upload.delete(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, upload = ?upload, "Unable to delete upload");
        InternalServerError(err)
    })?;

    let path = env.cache_dir.join(&upload.slug);
    tracing::info!(path = ?path, id = id, "Deleting cached upload");
    if let Err(err) = tokio::fs::remove_file(&path).await {
        tracing::error!(path = ?path, err = ?err, id = id, "Failed to delete cached upload");
    }

    let remaining = Upload::count_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, "Failed to count remaining uploads for user");
            InternalServerError(err)
        })?;

    Ok(Html(if remaining == 0 {
        "<tr><td colspan=\"9\" class=\"text-center italic\">No more uploads</td></tr>"
    } else {
        ""
    })
    .with_header("HX-Trigger", "parcelUploadDeleted")
    .into_response())
}
