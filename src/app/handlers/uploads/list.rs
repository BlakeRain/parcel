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
        handlers::utils::check_permission,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{
        team::{HomeTab, TeamTab},
        types::Key,
        upload::{Upload, UploadList, UploadOrder, UploadPermission, UploadStats},
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

    let home = HomeTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get home tab for user");
            poem::error::InternalServerError(err)
        })?;

    let tabs = TeamTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get team tabs for user");
            poem::error::InternalServerError(err)
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
            home,
            tabs,
            stats,
            uploads,
            has_teams,
            query,
            page => 0,
            limit => user.limit,
            ..authorized_context(&env, &user)
        },
    )
    .await
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

        check_permission(&env, &upload, Some(&user), UploadPermission::Delete).await?;

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
    .await
}
