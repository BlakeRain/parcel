use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{Data, Form, Html, Path, Query},
    IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{
        types::Key,
        upload::{Upload, UploadList, UploadOrder, UploadStats},
    },
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub order: UploadOrder,
    #[serde(default)]
    pub asc: bool,
}

#[handler]
pub async fn get_list(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let has_teams = user.has_teams(&env.pool).await.map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to check if user has teams");
        InternalServerError(err)
    })?;

    let stats = UploadStats::get_for_user(&env.pool, user.id)
        .await
        .map_err(InternalServerError)?;

    let uploads = UploadList::get_for_user(&env.pool, user.id, query.order, query.asc, 0, 50)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get uploads for user");
            InternalServerError(err)
        })?;

    render_template(
        "uploads/list.html",
        context! {
            stats,
            uploads,
            has_teams,
            query,
            page => 0,
            limit => user.limit,
            ..authorized_context(&env, &user)
        },
    )
}

#[handler]
pub async fn post_delete(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Form(form): Form<Vec<(String, Key<Upload>)>>,
) -> poem::Result<impl IntoResponse> {
    let ids = form
        .into_iter()
        .filter(|(name, _)| name == "selected")
        .map(|(_, id)| id)
        .collect::<Vec<_>>();

    for id in ids {
        let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
            tracing::error!(?err, ?id, "Unable to get upload by ID");
            InternalServerError(err)
        })?
        else {
            tracing::error!(id = ?id, "Unable to find upload with given ID");
            return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
        };

        let can_delete = user.admin || upload.is_owner(&env.pool, &user).await.map_err(|err| {
            tracing::error!(%upload.id, %user.id, ?err, "Unable to check if user is owner of an upload");
            InternalServerError(err)
        })?;

        if !can_delete {
            tracing::error!(
                %user.id,
                %upload.id,
                "User tried to delete upload without permission"
            );

            return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
        }

        upload.delete(&env.pool).await.map_err(|err| {
            tracing::error!(err = ?err, upload = ?upload, "Unable to delete upload");
            InternalServerError(err)
        })?;

        let path = env.cache_dir.join(&upload.slug);
        tracing::info!(?path, %id, "Deleting cached upload");
        if let Err(err) = tokio::fs::remove_file(&path).await {
            tracing::error!(?path, ?err, %id, "Failed to delete cached upload");
        }
    }

    Ok(Html("").with_header("HX-Refresh", "true"))
}

#[handler]
pub async fn get_page(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(page): Path<u32>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let has_teams = user.has_teams(&env.pool).await.map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to check if user has teams");
        InternalServerError(err)
    })?;

    let uploads =
        UploadList::get_for_user(&env.pool, user.id, query.order, query.asc, 50 * page, 50)
            .await
            .map_err(|err| {
                tracing::error!(%user.id, ?err, "Unable to get uploads for user");
                InternalServerError(err)
            })?;

    render_template(
        "uploads/page.html",
        context! {
            page,
            uploads,
            has_teams,
            query,
            ..authorized_context(&env, &user)
        },
    )
}
