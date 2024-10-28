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

use crate::{
    app::{
        extractors::user::SessionUser,
        handlers::utils::{check_permission, get_upload_by_slug},
    },
    env::Env,
    model::{
        upload::{Upload, UploadPermission},
        user::{verify_password, User},
    },
};

async fn send_download(
    env: &Env,
    mut upload: Upload,
    user: Option<&User>,
) -> poem::Result<Response> {
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
        .record_download(&env.pool, user)
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
    user: Option<SessionUser>,
    Path(slug): Path<String>,
) -> poem::Result<Response> {
    let mut upload = get_upload_by_slug(&env, &slug).await?;
    check_permission(
        &env,
        &upload,
        user.as_deref(),
        UploadPermission::Download {
            with_password: false,
        },
    )
    .await?;

    send_download(&env, upload, user.as_deref()).await
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

    let mut upload = get_upload_by_slug(&env, &slug).await?;
    check_permission(
        &env,
        &upload,
        user.as_deref(),
        UploadPermission::Download {
            with_password: true,
        },
    )
    .await?;

    let Some(ref hash) = upload.password else {
        return Ok(Redirect::see_other(format!("/uploads/{slug}/download")).into_response());
    };

    if !verify_password(hash, &password) {
        tracing::error!(%upload.id, "Invalid password provided");
        session.set("download_error", "Incorrect password");
        return Ok(Redirect::see_other(format!("/uploads/{slug}")).into_response());
    }

    send_download(&env, upload, user.as_deref()).await
}
