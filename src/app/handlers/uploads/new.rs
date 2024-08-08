use std::collections::HashMap;

use esbuild_bundle::javascript;
use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Html, Json, Multipart, RealIp},
};
use serde::Serialize;
use time::OffsetDateTime;

use crate::{
    app::templates::{authorized_context, render_template},
    env::Env,
    model::{upload::Upload, user::User},
};

#[handler]
pub fn get_new(env: Data<&Env>, csrf_token: &CsrfToken, user: User) -> poem::Result<Html<String>> {
    render_template(
        "uploads/new.html",
        context! {
            csrf_token => csrf_token.0,
            upload_js => javascript!("scripts/components/upload.ts"),
            ..authorized_context(&env, &user)
        },
    )
}

#[derive(Debug, Default, Serialize)]
pub struct UploadResult {
    uploads: HashMap<String, Option<Upload>>,
}

#[handler]
pub async fn post_new(
    env: Data<&Env>,
    RealIp(ip): RealIp,
    user: User,
    csrf_verifier: &CsrfVerifier,
    mut form: Multipart,
) -> poem::Result<Json<UploadResult>> {
    let mut seen_csrf = false;
    let mut uploads = Vec::new();
    let mut failures = Vec::new();

    while let Ok(Some(field)) = form.next_field().await {
        if field.name() == Some("csrf_token") {
            if !csrf_verifier.is_valid(&field.text().await?) {
                tracing::error!("CSRF token is invalid in upload form");
                return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
            }

            seen_csrf = true;
        } else if field.name() == Some("file") {
            let slug = nanoid::nanoid!();
            let filename = field
                .file_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| "unnamed.ext".to_string());

            let mut field = field.into_async_read();
            let path = env.cache_dir.join(&slug);

            {
                let mut file = tokio::fs::File::create(&path).await.map_err(|err| {
                    tracing::error!(err = ?err, path = ?path,
                                        "Unable to create file");
                    InternalServerError(err)
                })?;

                if let Err(err) = tokio::io::copy(&mut field, &mut file).await {
                    tracing::error!(err = ?err, path = ?path,
                                        "Unable to copy from stream to file");
                    failures.push(filename.clone());
                    continue;
                }
            }

            let meta = tokio::fs::metadata(&path).await.map_err(|err| {
                tracing::error!(err = ?err, path = ?path,
                                    "Unable to get metadata for file");
                InternalServerError(err)
            })?;

            let size = meta.len() as i64;
            tracing::info!(slug = slug, size = size, "Upload to cache complete");

            let mut upload = Upload {
                id: 0,
                slug,
                filename,
                size,
                public: false,
                downloads: 0,
                limit: None,
                remaining: None,
                expiry_date: None,
                uploaded_by: user.id,
                uploaded_at: OffsetDateTime::now_utc(),
                remote_addr: ip.as_ref().map(ToString::to_string),
            };

            uploads.push(upload);
        } else {
            tracing::info!(field_name = field.name(), "Ignoring unrecognized field");
        }
    }

    if !seen_csrf {
        tracing::error!("CSRF token was not seen in upload form");

        for upload in uploads {
            let path = env.cache_dir.join(&upload.slug);
            tracing::info!(path = ?path, slug = ?upload.slug, "Deleting cached upload");
            if let Err(err) = tokio::fs::remove_file(&path).await {
                tracing::error!(path = ?path, err = ?err, slug = ?upload.slug,
                        "Failed to delete cached upload");
            }
        }

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let mut result = UploadResult::default();
    for mut upload in uploads {
        upload.create(&env.pool).await.map_err(|err| {
            tracing::error!(err = ?err, "Unable to create upload");
            InternalServerError(err)
        })?;

        tracing::info!(upload = ?upload, "Created upload");
        result.uploads.insert(upload.filename.clone(), Some(upload));
    }

    for filename in failures {
        result.uploads.insert(filename, None);
    }

    Ok(Json(result))
}
