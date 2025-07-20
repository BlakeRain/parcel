use esbuild_bundle::javascript;
use poem::{
    handler,
    web::{CsrfToken, Data, Html, Query},
    IntoResponse, Response,
};

use crate::{
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{
        team::{HomeTab, TeamTab},
        upload::{UploadList, UploadStats},
    },
};

use super::uploads::ListQuery;

#[handler]
pub async fn get_index(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_token: &CsrfToken,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
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

    // Get the stats for the user's own uploads.
    let stats = UploadStats::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get stats for user");
            poem::error::InternalServerError(err)
        })?;

    // Get the first page of uploads for the user.
    let uploads = UploadList::get_for_user(
        &env.pool,
        user.id,
        query.get_search(),
        query.order.unwrap_or(user.default_order),
        query.asc.unwrap_or(user.default_asc),
        0,
        50,
    )
    .await
    .map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to get uploads for user");
        poem::error::InternalServerError(err)
    })?;

    render_template(
        "index.html",
        minijinja::context! {
            query,
            home,
            tabs,
            stats,
            uploads,
            csrf_token => csrf_token.0,
            page => 0,
            limit => user.limit,
            index_js => javascript!("scripts/index.ts"),
            ..authorized_context(&env, &user)
        },
    )
    .await
}

#[handler]
pub async fn get_tab(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Query(query): Query<ListQuery>,
) -> poem::Result<Response> {
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
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get stats for user");
            poem::error::InternalServerError(err)
        })?;

    let uploads = UploadList::get_for_user(
        &env.pool,
        user.id,
        query.get_search(),
        query.order.unwrap_or(user.default_order),
        query.asc.unwrap_or(user.default_asc),
        0,
        50,
    )
    .await
    .map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to get uploads for user");
        poem::error::InternalServerError(err)
    })?;

    Ok(render_template(
        "tab.html",
        minijinja::context! {
            query,
            home,
            tabs,
            stats,
            uploads,
            page => 0,
            limit => user.limit,
            ..authorized_context(&env, &user)
        },
    )
    .await?
    .with_header("HX-Push-Url", "/")
    .into_response())
}
