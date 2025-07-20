use esbuild_bundle::javascript;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{Data, Html, Path, Query},
    IntoResponse, Response,
};

use crate::{
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{
        team::{HomeTab, Team, TeamMember, TeamTab},
        upload::{UploadList, UploadStats},
    },
};

use super::uploads::ListQuery;

pub mod uploads;

#[handler]
pub async fn get_team(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(slug): Path<String>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let Some(team) = Team::get_by_slug(&env.pool, &slug).await.map_err(|err| {
        tracing::error!(?slug, ?err, "Unable to get team by ID or slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(?slug, "Team with this URL slug does not exist");
        return Err(poem::Error::from_status(poem::http::StatusCode::NOT_FOUND));
    };

    let Some(membership) = TeamMember::get_for_user_and_team(&env.pool, user.id, team.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, %team.id, ?err, "Unable to get team membership");
            InternalServerError(err)
        })?
    else {
        tracing::error!(%user.id, %team.id, "User is not a member of team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    };

    let home = HomeTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get home tab for user");
            InternalServerError(err)
        })?;

    let tabs = TeamTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get team tabs for user");
            InternalServerError(err)
        })?;

    let stats = UploadStats::get_for_team(&env.pool, team.id)
        .await
        .map_err(InternalServerError)?;

    let uploads = UploadList::get_for_team(
        &env.pool,
        team.id,
        query.get_search(),
        query.order,
        query.asc,
        0,
        50,
    )
    .await
    .map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to get uploads for team");
        InternalServerError(err)
    })?;

    render_template(
        "team.html",
        minijinja::context! {
            query,
            tabs,
            home,
            team,
            membership,
            stats,
            uploads,
            page => 0,
            limit => team.limit,
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
    Path(slug): Path<String>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Response> {
    let Some(team) = Team::get_by_slug(&env.pool, &slug).await.map_err(|err| {
        tracing::error!(?slug, ?err, "Unable to get team by ID or slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(?slug, "Team with this URL slug does not exist");
        return Err(poem::Error::from_status(poem::http::StatusCode::NOT_FOUND));
    };

    let Some(membership) = TeamMember::get_for_user_and_team(&env.pool, user.id, team.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, %team.id, ?err, "Unable to get team membership");
            InternalServerError(err)
        })?
    else {
        tracing::error!(%user.id, %team.id, "User is not a member of team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    };

    let home = HomeTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get home tab for user");
            InternalServerError(err)
        })?;

    let tabs = TeamTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get team tabs for user");
            InternalServerError(err)
        })?;

    let stats = UploadStats::get_for_team(&env.pool, team.id)
        .await
        .map_err(InternalServerError)?;

    let uploads = UploadList::get_for_team(
        &env.pool,
        team.id,
        query.get_search(),
        query.order,
        query.asc,
        0,
        50,
    )
    .await
    .map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to get uploads for team");
        InternalServerError(err)
    })?;

    Ok(render_template(
        "tab.html",
        minijinja::context! {
            query,
            tabs,
            home,
            team,
            membership,
            stats,
            uploads,
            page => 0,
            limit => team.limit,
            ..authorized_context(&env, &user)
        },
    )
    .await?
    .with_header("HX-Push-Url", format!("/teams/{}", team.slug))
    .into_response())
}
