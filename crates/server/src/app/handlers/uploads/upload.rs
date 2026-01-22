
use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::{
        header::{CONTENT_LENGTH, CONTENT_TYPE},
        StatusCode,
    },
    session::Session,
    web::{CsrfToken, CsrfVerifier, Data, Html, Path, Query},
    IntoResponse,
};
use serde::Deserialize;
use time::OffsetDateTime;

use parcel_model::{
    team::{HomeTab, Team, TeamMember, TeamTab},
    types::Key,
    upload::{Upload, UploadPermission, UploadStats},
    user::User,
};

use crate::{
    app::{
        errors::CsrfError,
        extractors::user::SessionUser,
        handlers::utils::{
            check_permission, delete_upload_cache, get_upload_by_id, get_upload_by_slug,
        },
        templates::{authorized_context, default_context, render_template},
    },
    env::Env,
    utils::SessionExt,
};

async fn render_upload(
    env: Data<&Env>,
    user: Option<&User>,
    session: &Session,
    csrf_token: &CsrfToken,
    upload: Upload,
) -> poem::Result<Html<String>> {
    check_permission(&env, &upload, user, UploadPermission::View).await?;

    let owner = if let Some(user) = user {
        upload
            .is_owner(&env.pool, user)
            .await
            .map_err(InternalServerError)?
            .is_some()
    } else {
        false
    };

    let membership = if let Some(user) = user {
        if let Some(team_id) = upload.owner_team {
            TeamMember::get_for_user_and_team(&env.pool, user.id, team_id)
                .await
                .map_err(InternalServerError)?
        } else {
            None
        }
    } else {
        None
    };

    let uploader =
        if let Some(uploaded_by) = upload.uploaded_by {
            Some(User::get(&env.pool, uploaded_by)
            .await
            .map_err(|err| {
                tracing::error!(?err, user_id = %uploaded_by, "Unable to get user by ID");
                InternalServerError(err)
            })?
            .ok_or_else(|| {
                tracing::error!(user_id = %uploaded_by, "Unable to find user with given ID");
                poem::Error::from_status(StatusCode::NOT_FOUND)
            })?)
        } else {
            None
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
            membership,
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
    .await
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

#[derive(Debug, Deserialize)]
pub struct DeleteUploadQuery {
    csrf_token: String,
}

#[handler]
pub async fn delete_upload(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_verifier: &CsrfVerifier,
    Path(id): Path<Key<Upload>>,
    Query(DeleteUploadQuery { csrf_token }): Query<DeleteUploadQuery>,
) -> poem::Result<poem::Response> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::warn!(%user.id, %id, "CSRF token verification failed for upload deletion");
        return Err(CsrfError.into());
    }

    let upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::Delete).await?;

    upload.delete(&env.pool).await.map_err(|err| {
        tracing::error!(?err, %upload.id, "Unable to delete upload");
        InternalServerError(err)
    })?;

    delete_upload_cache(&env, &upload).await;

    let (stats, limit, team) = match (upload.owner_user, upload.owner_team) {
        (Some(user_id), None) => {
            if user_id != user.id {
                tracing::error!(%user_id, %user.id, "User ID mismatch after permission check");
                return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
            }
            let stats = UploadStats::get_for_user(&env.pool, user.id)
                .await
                .map_err(InternalServerError)?;
            (stats, user.limit, None)
        }

        (None, Some(team_id)) => {
            let stats = UploadStats::get_for_team(&env.pool, team_id)
                .await
                .map_err(InternalServerError)?;
            let Some(team) = Team::get(&env.pool, team_id)
                .await
                .map_err(InternalServerError)?
            else {
                tracing::error!(team_id = %team_id, "Unable to find team with given ID");
                return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
            };

            (stats, team.limit, Some(team))
        }

        (_, _) => {
            tracing::error!(%upload.id, "Upload has invalid ownership state");
            return Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
        }
    };

    let home = HomeTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get home tab for user");
            poem::error::InternalServerError(err)
        })?;

    let tabs = TeamTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get team tabs for user");
            poem::error::InternalServerError(err)
        })?;

    Ok(render_template(
        "uploads/deleted.html",
        context! {
            team,
            home,
            tabs,
            stats,
            limit,
            ..authorized_context(&env, &user)
        },
    )
    .await?
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
    csrf_token: &CsrfToken,
    Path(id): Path<Key<Upload>>,
    Query(GetShareQuery { immediate }): Query<GetShareQuery>,
) -> poem::Result<Html<String>> {
    let upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::Share).await?;

    let team = if let Some(team_id) = upload.owner_team {
        Team::get(&env.pool, team_id).await.map_err(|err| {
            tracing::error!(?err, team_id = %team_id, "Unable to get team by ID");
            InternalServerError(err)
        })?
    } else {
        None
    };

    render_template(
        "uploads/share.html",
        context! {
            upload,
            immediate,
            team,
            csrf_token => csrf_token.0,
            ..authorized_context(&env, &user)
        },
    )
    .await
}

#[handler]
pub async fn get_preview(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
) -> poem::Result<poem::Response> {
    let upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::View).await?;

    if !upload.has_preview {
        tracing::warn!(%upload.id, "Upload does not have a preview");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    }

    let path = env.cache_dir.join(format!("{}.preview", upload.slug));
    let file = tokio::fs::File::open(&path).await.map_err(|err| {
        tracing::error!(%upload.id, ?err, ?path, "Unable to open file");
        InternalServerError(err)
    })?;

    let meta = file.metadata().await.map_err(|err| {
        tracing::error!(%upload.id, ?err, ?path, "Unable to get metadata for file");
        InternalServerError(err)
    })?;

    Ok(poem::Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "image/png")
        .header(CONTENT_LENGTH, meta.len())
        .body(poem::Body::from_async_read(file)))
}

#[derive(Debug, Deserialize)]
pub struct DeletePreviewErrorQuery {
    csrf_token: String,
}

#[handler]
pub async fn delete_preview_error(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_verifier: &CsrfVerifier,
    Path(id): Path<Key<Upload>>,
    Query(DeletePreviewErrorQuery { csrf_token }): Query<DeletePreviewErrorQuery>,
) -> poem::Result<poem::Response> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::warn!(%user.id, %id, "CSRF token verification failed for upload deletion");
        return Err(CsrfError.into());
    }

    let mut upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::Edit).await?;

    upload.clear_preview_error(&env.pool).await.map_err(|err| {
        tracing::error!(?err, %upload.id, "Unable to clear preview error for upload");
        InternalServerError(err)
    })?;

    Ok(Html("")
        .with_header(
            "HX-Trigger",
            serde_json::json!({
                "parcelUploadChanged": id,
            })
            .to_string(),
        )
        .into_response())
}
