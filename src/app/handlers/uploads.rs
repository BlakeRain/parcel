use poem::{
    error::InternalServerError,
    handler,
    web::{Data, Html, Path, Query},
    IntoResponse, Response,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    app::{
        extractors::user::SessionUser,
        handlers::utils::{check_owns_upload, get_upload_by_id},
    },
    env::Env,
    model::{types::Key, upload::Upload},
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
pub use upload::{delete_upload, get_custom_upload, get_share, get_upload};

#[derive(Debug, Deserialize)]
pub struct MakePublicQuery {
    public: bool,
}

#[handler]
pub async fn post_public(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
    Query(MakePublicQuery { public }): Query<MakePublicQuery>,
) -> poem::Result<Response> {
    let mut upload = get_upload_by_id(&env, id).await?;
    check_owns_upload(&env, &user, &upload).await?;

    tracing::info!(%upload.id, public, "Setting upload public state");
    upload
        .set_public(&env.pool, public)
        .await
        .map_err(InternalServerError)?;

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

#[handler]
pub async fn post_reset(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
) -> poem::Result<Response> {
    let mut upload = get_upload_by_id(&env, id).await?;
    check_owns_upload(&env, &user, &upload).await?;

    tracing::info!(%upload.id, "Resetting upload download stats");
    upload
        .reset_remaining(&env.pool)
        .await
        .map_err(InternalServerError)?;

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
