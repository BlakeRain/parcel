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
    env::Env,
    model::{
        upload::Upload,
        user::{verify_password, User},
    },
};

async fn send_download(env: &Env, owner: bool, mut upload: Upload) -> poem::Result<Response> {
    let path = env.cache_dir.join(&upload.slug);
    tracing::info!(upload = upload.id, path = ?path, "Opening file for upload");
    let file = tokio::fs::File::open(&path).await.map_err(|err| {
        tracing::error!(err = ?err, path = ?path, "Unable to open file");
        InternalServerError(err)
    })?;

    let meta = file.metadata().await.map_err(|err| {
        tracing::error!(err = ?err, path = ?path, "Unable to get metadata for file");
        InternalServerError(err)
    })?;

    upload
        .record_download(&env.pool, !owner)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, slug = ?upload.slug, "Unable to record download");
            InternalServerError(err)
        })?;

    tracing::info!(upload = upload.id, meta = ?meta,
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
    user: Option<User>,
    Path(slug): Path<String>,
) -> poem::Result<Response> {
    let Some(mut upload) = Upload::get_by_slug(&env.pool, &slug).await.map_err(|err| {
        tracing::error!(err = ?err, slug = ?slug, "Unable to get upload by slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(slug = ?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

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

    if !owner {
        if let Some(remaining) = upload.remaining {
            if remaining < 1 {
                tracing::error!(upload = ?upload, "Download limit was reached");
                return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
            }
        }

        if let Some(expiry) = upload.expiry_date {
            if expiry < OffsetDateTime::now_utc().date() {
                tracing::error!(upload = ?upload, "Upload has expired");
                return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
            }
        }

        if upload.password.is_some() {
            tracing::error!(upload = ?upload, "Upload requires password");
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
    user: Option<User>,
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
        tracing::error!(err = ?err, slug = ?slug, "Unable to get upload by slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(slug = ?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let owner = if let Some(user) = &user {
        user.admin || upload.uploaded_by == user.id
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
        tracing::error!(upload = ?upload, "Invalid password provided");
        session.set("download_error", "Incorrect password");
        return Ok(Redirect::see_other(format!("/uploads/{slug}")).into_response());
    }

    send_download(&env, owner, upload).await
}
