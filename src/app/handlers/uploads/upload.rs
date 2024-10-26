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
        handlers::utils::{check_owns_upload, get_upload_by_id, get_upload_by_slug},
        templates::{authorized_context, default_context, render_template},
    },
    env::Env,
    model::{team::Team, types::Key, upload::Upload, user::User},
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

    let team = if let Some(team_id) = upload.owner_team {
        let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
            tracing::error!(?err, team_id = %team_id, "Unable to get team by ID");
            InternalServerError(err)
        })?
        else {
            tracing::error!(team_id = %team_id, "Unable to find team with given ID");
            return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
        };

        Some(team)
    } else {
        None
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
            team,
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
    let upload = get_upload_by_slug(&env, &slug).await?;
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
    let Some(upload) = Upload::get_by_custom_slug(&env.pool, &owner, &slug)
        .await
        .map_err(|err| {
            tracing::error!(?owner, ?slug, ?err, "Unable to get upload by custom slug");
            InternalServerError(err)
        })?
    else {
        tracing::error!(
            ?owner,
            ?slug,
            "Unable to find upload with given custom slug"
        );
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
    let upload = get_upload_by_id(&env, id).await?;
    check_owns_upload(&env, &user, &upload).await?;

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
    let upload = get_upload_by_id(&env, id).await?;
    check_owns_upload(&env, &user, &upload).await?;
    render_template(
        "uploads/share.html",
        context! {
            upload,
            immediate,
            ..authorized_context(&env, &user)
        },
    )
}
