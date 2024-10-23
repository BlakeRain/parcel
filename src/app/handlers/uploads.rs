use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{Data, Html, Path, Query},
    IntoResponse, Response,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    app::{extractors::user::SessionUser, templates::render_404},
    env::Env,
    model::{types::Key, upload::Upload},
};

mod download;
mod edit;
mod list;
mod new;
mod upload;

pub use download::{get_download, post_download};
pub use edit::{get_edit, post_check_slug, post_edit};
pub use list::{get_list, get_page, post_delete, ListQuery};
pub use new::{get_new, post_new};
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
    let Some(mut upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(?err, %id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return render_404("Unrecognized upload ID").map(IntoResponse::into_response);
    };

    let can_modify = user.admin || upload.is_owner(&env.pool, &user).await.map_err(|err| {
        tracing::error!(%upload.id, %user.id, ?err, "Unable to check if user is owner of an upload");
        InternalServerError(err)
    })?;

    if !can_modify {
        tracing::error!(
            %user.id,
            %upload.id,
            "User tried to edit upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

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
    let Some(mut upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(?err, %id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
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
            "User tried to reset upload without permission"
        );
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

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
