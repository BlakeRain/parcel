use poem::{
    error::InternalServerError,
    handler,
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
        StatusCode,
    },
    session::Session,
    web::{CsrfVerifier, Data, Form, Path, Redirect},
    Body, IntoResponse, Response,
};
use serde::Deserialize;

use parcel_model::{
    upload::{Upload, UploadPermission},
    user::User,
};

use crate::{
    app::{
        errors::CsrfError,
        extractors::user::SessionUser,
        handlers::utils::{check_permission, get_upload_by_slug},
    },
    env::Env,
};

/// Builds a Content-Disposition header value with a safely encoded filename.
///
/// This function properly escapes the filename to prevent header injection attacks
/// and uses RFC 5987 encoding for non-ASCII characters.
fn content_disposition_filename(filename: &str) -> String {
    // Check if filename contains only ASCII characters
    let is_ascii = filename.chars().all(|c| c.is_ascii() && c != '"' && c != '\\');

    if is_ascii {
        // Simple case: ASCII-only filename, just need to escape quotes and backslashes
        let escaped = filename.replace('\\', "\\\\").replace('"', "\\\"");
        format!("attachment; filename=\"{escaped}\"")
    } else {
        // Use RFC 5987 encoding for non-ASCII filenames
        // Also provide a fallback ASCII filename
        let ascii_fallback: String = filename
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
            .collect();

        // Percent-encode the UTF-8 filename for filename*
        let encoded: String = filename
            .as_bytes()
            .iter()
            .map(|&byte| {
                if byte.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&byte) {
                    (byte as char).to_string()
                } else {
                    format!("%{:02X}", byte)
                }
            })
            .collect();

        format!("attachment; filename=\"{ascii_fallback}\"; filename*=UTF-8''{encoded}")
    }
}

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

    tracing::info!(%upload.id, ?meta, "Sending file to client");

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_DISPOSITION, content_disposition_filename(&upload.filename))
        .header(CONTENT_LENGTH, meta.len());

    if let Some(ref mime_type) = upload.mime_type {
        builder = builder.header(CONTENT_TYPE, mime_type);
    }

    Ok(builder.body(Body::from_async_read(file)))
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
        return Err(CsrfError.into());
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

    if !hash.verify(&password) {
        tracing::error!(%upload.id, "Invalid password provided");
        session.set("download_error", "Incorrect password");
        return Ok(Redirect::see_other(format!("/uploads/{slug}")).into_response());
    }

    if hash.needs_migrating() {
        tracing::info!(%upload.id, "Migrating password hash");
        upload.set_password(&env.pool, &password).await?;
    }

    send_download(&env, upload, user.as_deref()).await
}
