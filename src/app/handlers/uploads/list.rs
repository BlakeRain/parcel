use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{Data, Form, Html, Query},
    IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    app::templates::{authorized_context, render_template},
    env::Env,
    model::{
        upload::{Upload, UploadOrder},
        user::User,
    },
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub order: UploadOrder,
    #[serde(default)]
    pub asc: bool,
    #[serde(default)]
    pub page: i32,
}

#[handler]
pub async fn get_list(
    env: Data<&Env>,
    user: User,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let total = Upload::count_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(user = user.id, err = ?err, "Unable to get upload count for user");
            InternalServerError(err)
        })?;

    let offset = query.page * 100;
    let uploads = Upload::get_for_user(&env.pool, user.id, query.order, query.asc, offset, 100)
        .await
        .map_err(|err| {
            tracing::error!(user = user.id, err = ?err, "Unable to get uploads for user");
            InternalServerError(err)
        })?;

    render_template(
        "uploads/list.html",
        context! {
            total,
            uploads,
            query,
            pages => (0..total / 100 + 1).collect::<Vec<_>>(),
            ..authorized_context(&env, &user)
        },
    )
}

#[handler]
pub async fn delete_list(
    env: Data<&Env>,
    user: User,
    Form(form): Form<Vec<(String, i32)>>,
) -> poem::Result<impl IntoResponse> {
    let ids = form
        .into_iter()
        .filter(|(name, _)| name == "selected")
        .map(|(_, id)| id)
        .collect::<Vec<_>>();

    for id in ids {
        let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
            tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
            InternalServerError(err)
        })?
        else {
            tracing::error!(id = ?id, "Unable to find upload with given ID");
            return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
        };

        if !user.admin && upload.uploaded_by != user.id {
            tracing::error!(
                user = user.id,
                upload = upload.id,
                "User tried to delete upload without permission"
            );

            return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
        }

        upload.delete(&env.pool).await.map_err(|err| {
            tracing::error!(err = ?err, upload = ?upload, "Unable to delete upload");
            InternalServerError(err)
        })?;

        let path = env.cache_dir.join(&upload.slug);
        tracing::info!(path = ?path, id = id, "Deleting cached upload");
        if let Err(err) = tokio::fs::remove_file(&path).await {
            tracing::error!(path = ?path, err = ?err, id = id, "Failed to delete cached upload");
        }
    }

    Ok(Html("").with_header("HX-Refresh", "true"))
}
