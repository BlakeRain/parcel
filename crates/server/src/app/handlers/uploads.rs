use poem::{
    error::InternalServerError,
    handler,
    web::{CsrfVerifier, Data, Form, Html, Path},
    IntoResponse, Response,
};
use serde::Deserialize;
use serde_json::json;

use parcel_model::{
    types::Key,
    upload::{Upload, UploadPermission},
};

use crate::{
    app::{
        errors::CsrfError,
        extractors::user::SessionUser,
        handlers::utils::{check_permission, get_upload_by_id},
    },
    env::Env,
};

mod download;
mod edit;
mod list;
mod new;
mod transfer;
mod upload;

pub use download::{get_download, post_download};
pub use edit::{get_edit, post_check_slug, post_edit};
pub use list::{get_list, get_page, post_delete, ListQuery};
pub use new::{get_new, post_new};
pub use transfer::{get_transfer, post_transfer};
pub use upload::{
    delete_preview_error, delete_upload, get_custom_upload, get_preview, get_share, get_upload,
};

#[derive(Debug, Deserialize)]
pub struct MakePublicQuery {
    public: bool,
    csrf_token: String,
}

#[handler]
pub async fn post_public(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_verifier: &CsrfVerifier,
    Path(id): Path<Key<Upload>>,
    Form(MakePublicQuery { public, csrf_token }): Form<MakePublicQuery>,
) -> poem::Result<Response> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::warn!(%id, "CSRF verification failed for upload public state change");
        return Err(CsrfError.into());
    }

    let mut upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::Edit).await?;

    tracing::info!(%upload.id, public, "Setting upload public state");
    upload
        .set_public(&env.pool, public)
        .await
        .map_err(|err| {
            tracing::error!(?err, %upload.id, "Failed to set upload public state");
            InternalServerError(err)
        })?;

    Ok(Html("")
        .with_header(
            "HX-Trigger",
            json!({
                "parcelUploadChanged": id,
            })
            .to_string(),
        )
        .into_response())
}

#[derive(Debug, Deserialize)]
pub struct ResetForm {
    csrf_token: String,
}

#[handler]
pub async fn post_reset(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_verifier: &CsrfVerifier,
    Path(id): Path<Key<Upload>>,
    Form(ResetForm { csrf_token }): Form<ResetForm>,
) -> poem::Result<Response> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::warn!(%id, "CSRF verification failed for upload reset");
        return Err(CsrfError.into());
    }

    let mut upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::ResetDownloads).await?;

    tracing::info!(%upload.id, "Resetting upload download stats");
    upload
        .reset_remaining(&env.pool)
        .await
        .map_err(|err| {
            tracing::error!(?err, %upload.id, "Failed to reset upload remaining downloads");
            InternalServerError(err)
        })?;

    Ok(Html("")
        .with_header(
            "HX-Trigger",
            json!({
                "parcelUploadChanged": id,
            })
            .to_string(),
        )
        .into_response())
}
