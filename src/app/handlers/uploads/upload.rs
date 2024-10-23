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
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, default_context, render_template},
    },
    env::Env,
    model::{types::Key, upload::Upload, user::User},
    utils::SessionExt,
};

async fn render_upload(
    env: Data<&Env>,
    user: Option<&User>,
    session: &Session,
    csrf_token: &CsrfToken,
    upload: Upload,
) -> poem::Result<Html<String>> {
    let owner = if let Some(user) = &user {
        user.admin
            || upload.is_owner(&env.pool, user).await.map_err(|err| {
                tracing::error!(%user.id, ?err, "Unable to check if user is owner of an upload");
                InternalServerError(err)
            })?
    } else {
        false
    };

    if !upload.public && !owner {
        let uid = user.as_ref().map(|u| u.id.to_string());
        tracing::error!(
            user = ?uid,
            upload = %upload.id,
            "User tried to access private upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    }

    let Some(uploader) = User::get(&env.pool, upload.uploaded_by)
        .await
        .map_err(|err| {
            tracing::error!(?err, user_id = %upload.uploaded_by,
                            "Unable to get user by ID");
            InternalServerError(err)
        })?
    else {
        tracing::error!(user_id = %upload.uploaded_by, "Unable to find user with given ID");
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
    user: Option<SessionUser>,
    csrf_token: &CsrfToken,
    Path(slug): Path<String>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get_by_slug(&env.pool, &slug).await.map_err(|err| {
        tracing::error!(?err, ?slug, "Unable to get upload by slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    render_upload(env, user.as_deref(), session, csrf_token, upload).await
}

#[handler]
pub async fn get_custom_upload(
    env: Data<&Env>,
    session: &Session,
    user: Option<SessionUser>,
    csrf_token: &CsrfToken,
    Path((owner, slug)): Path<(String, String)>,
) -> poem::Result<Html<String>> {
    let Some(owner) = User::get_by_username(&env.pool, &owner)
        .await
        .map_err(|err| {
            tracing::error!(?err, ?owner, "Unable to get user by username");
            InternalServerError(err)
        })?
    else {
        tracing::error!(?owner, "Unable to find user with given username");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let Some(upload) = Upload::get_by_custom_slug(&env.pool, owner.id, &slug)
        .await
        .map_err(|err| {
            tracing::error!(?err, ?slug, "Unable to get upload by slug");
            InternalServerError(err)
        })?
    else {
        tracing::error!(?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    render_upload(env, user.as_deref(), session, csrf_token, upload).await
}

#[handler]
pub async fn delete_upload(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
) -> poem::Result<poem::Response> {
    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(?err, %id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%id, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let can_delete = user.admin || upload.is_owner(&env.pool, &user).await.map_err(|err| {
        tracing::error!(%upload.id, %user.id, ?err, "Unable to check if user is owner of an upload");
        InternalServerError(err)
    })?;

    if !can_delete {
        tracing::error!(
            user = %user.id,
            upload = %upload.id,
            "User tried to delete upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    upload.delete(&env.pool).await.map_err(|err| {
        tracing::error!(?err, %upload.id, "Unable to delete upload");
        InternalServerError(err)
    })?;

    let path = env.cache_dir.join(&upload.slug);
    tracing::info!(?path, %upload.id, "Deleting cached upload");
    if let Err(err) = tokio::fs::remove_file(&path).await {
        tracing::error!(?path, ?err, %upload.id, "Failed to delete cached upload");
    }

    Ok(Html("")
        .with_header("HX-Trigger", "parcelUploadDeleted")
        .into_response())
}

#[derive(Debug, Deserialize)]
pub struct GetShareQuery {
    #[serde(default)]
    immediate: bool,
}

#[handler]
pub async fn get_share(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
    Query(GetShareQuery { immediate }): Query<GetShareQuery>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(?err, %id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%id, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let can_modify = user.admin || upload.is_owner(&env.pool, &user).await.map_err(|err| {
        tracing::error!(%upload.id, %user.id, ?err, "Unable to check if user is owner of an upload");
        InternalServerError(err)
    })?;

    if !can_modify {
        tracing::error!(
            %user.id,
            %upload.id,
            "User tried to share an upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    render_template(
        "uploads/share.html",
        context! {
            upload,
            immediate,
            ..authorized_context(&env, &user)
        },
    )
}
