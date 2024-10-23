use poem::{
    error::InternalServerError,
    handler,
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH},
        StatusCode,
    },
    session::Session,
    web::{CsrfVerifier, Data, Form, Path, Redirect},
    Body, IntoResponse, Response,
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    app::extractors::user::SessionUser,
    env::Env,
    model::{upload::Upload, user::verify_password},
};

async fn send_download(env: &Env, owner: bool, mut upload: Upload) -> poem::Result<Response> {
    let path = env.cache_dir.join(&upload.slug);
    tracing::info!(upload = %upload.id, path = ?path, "Opening file for upload");
    let file = tokio::fs::File::open(&path).await.map_err(|err| {
        tracing::error!(%upload.id, ?err, ?path, "Unable to open file");
        InternalServerError(err)
    })?;

    let meta = file.metadata().await.map_err(|err| {
        tracing::error!(%upload.id, ?err, ?path, "Unable to get metadata for file");
        InternalServerError(err)
    })?;

    upload
        .record_download(&env.pool, !owner)
        .await
        .map_err(|err| {
            tracing::error!(%upload.id, ?err, ?upload.slug, "Unable to record download");
            InternalServerError(err)
        })?;

    tracing::info!(%upload.id, ?meta,
                   "Sending file to client");

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", upload.filename),
        )
        .header(CONTENT_LENGTH, meta.len())
        .body(Body::from_async_read(file)))
}

#[handler]
pub async fn get_download(
    env: Data<&Env>,
    session: &Session,
    user: Option<SessionUser>,
    Path(slug): Path<String>,
) -> poem::Result<Response> {
    let Some(mut upload) = Upload::get_by_slug(&env.pool, &slug).await.map_err(|err| {
        tracing::error!(?err, ?slug, "Unable to get upload by slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let owner = if let Some(SessionUser(user)) = &user {
        user.admin
            || upload.is_owner(&env.pool, user).await.map_err(|err| {
                tracing::error!(%user.id, ?err, "Unable to check if user is owner of an upload");
                InternalServerError(err)
            })?
    } else {
        false
    };

    if !upload.public && !owner {
        let uid = user.as_ref().map(|user| user.id);
        tracing::error!(
            user = ?uid,
            %upload.id,
            "User tried to access private upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    }

    if !owner {
        if let Some(remaining) = upload.remaining {
            if remaining < 1 {
                tracing::error!(upload = ?upload, "Download limit was reached");
                return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
            }
        }

        if let Some(expiry) = upload.expiry_date {
            if expiry < OffsetDateTime::now_utc().date() {
                tracing::error!(%upload.id, "Upload has expired");
                return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
            }
        }

        if upload.password.is_some() {
            tracing::error!(%upload.id, "Upload requires password");
            session.set("download_error", "This upload requires a password");
            return Ok(Redirect::see_other(format!("/uploads/{slug}")).into_response());
        }
    }

    send_download(&env, owner, upload).await
}

#[derive(Debug, Deserialize)]
pub struct DownloadForm {
    csrf_token: String,
    password: String,
}

#[handler]
pub async fn post_download(
    env: Data<&Env>,
    session: &Session,
    user: Option<SessionUser>,
    verifier: &CsrfVerifier,
    Path(slug): Path<String>,
    Form(DownloadForm {
        csrf_token,
        password,
    }): Form<DownloadForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&csrf_token) {
        tracing::error!("CSRF token is invalid in upload edit");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(mut upload) = Upload::get_by_slug(&env.pool, &slug).await.map_err(|err| {
        tracing::error!(?err, ?slug, "Unable to get upload by slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let owner = if let Some(SessionUser(user)) = &user {
        user.admin
            || upload.is_owner(&env.pool, user).await.map_err(|err| {
                tracing::error!(%user.id, ?err, "Unable to check if user is owner of an upload");
                InternalServerError(err)
            })?
    } else {
        false
    };

    if owner {
        return Ok(Redirect::see_other(format!("/uploads/{slug}/download")).into_response());
    }

    let Some(ref hash) = upload.password else {
        return Ok(Redirect::see_other(format!("/uploads/{slug}/download")).into_response());
    };

    if !verify_password(hash, &password) {
        tracing::error!(%upload.id, "Invalid password provided");
        session.set("download_error", "Incorrect password");
        return Ok(Redirect::see_other(format!("/uploads/{slug}")).into_response());
    }

    send_download(&env, owner, upload).await
}
