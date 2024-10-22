use minijinja::context;
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
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, render_404, render_template},
    },
    env::Env,
    model::{
        types::Key,
        upload::{Upload, UploadStats},
    },
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

#[handler]
pub async fn get_stats(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
) -> poem::Result<Html<String>> {
    let stats = UploadStats::get_for_user(&env.pool, user.id)
        .await
        .map_err(InternalServerError)?;

    // Generate a random int between 10 and 90
    let random = rand::random::<i8>() % 80 + 10;

    render_template(
        "uploads/stats.html",
        context! {
            stats,
            random,
            ..authorized_context(&env, &user)
        },
    )
}

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

    if !user.admin && upload.uploaded_by != user.id {
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

    if !user.admin && upload.uploaded_by != user.id {
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
