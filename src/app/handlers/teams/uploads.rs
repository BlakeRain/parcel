use minijinja::context;
use poem::{
    error::InternalServerError,
    http::StatusCode,
    web::{CsrfToken, Data, Html, Path, Query},
};

use crate::{
    app::{
        extractors::user::SessionUser,
        handlers::uploads::ListQuery,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{
        team::{HomeTab, Team, TeamMember, TeamTab},
        types::Key,
        upload::{UploadList, UploadStats},
    },
};

#[poem::handler]
pub async fn get_list(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_token: &CsrfToken,
    Path(team_id): Path<Key<Team>>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(%team_id, ?err, "Unable to get team");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%team_id, "Team not found");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let Some(membership) = TeamMember::get_for_user_and_team(&env.pool, user.id, team_id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, %team_id, ?err, "Unable to get team membership");
            InternalServerError(err)
        })?
    else {
        tracing::error!(%user.id, %team_id, "User is not a member of team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    };

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

    let stats = UploadStats::get_for_team(&env.pool, team_id)
        .await
        .map_err(InternalServerError)?;

    let uploads = UploadList::get_for_team(&env.pool, team.id, query.order, query.asc, 0, 50)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get uploads for team");
            InternalServerError(err)
        })?;

    render_template(
        "uploads/list.html",
        context! {
            team,
            membership,
            home,
            tabs,
            stats,
            uploads,
            query,
            csrf_token => csrf_token.0,
            page => 0,
            limit => team.limit,
            ..authorized_context(&env, &user)
        },
    )
    .await
}

#[poem::handler]
pub async fn get_page(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path((team_id, page)): Path<(Key<Team>, u32)>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(%team_id, ?err, "Unable to get team");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%team_id, "Team not found");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let Some(membership) = TeamMember::get_for_user_and_team(&env.pool, user.id, team_id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, %team_id, ?err, "Unable to get team membership");
            InternalServerError(err)
        })?
    else {
        tracing::error!(%user.id, %team_id, "User is not a member of team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    };

    let uploads =
        UploadList::get_for_team(&env.pool, team.id, query.order, query.asc, 50 * page, 50)
            .await
            .map_err(|err| {
                tracing::error!(%team.id, ?err, "Unable to get uploads for team");
                InternalServerError(err)
            })?;

    render_template(
        "uploads/page.html",
        context! {
            page,
            uploads,
            query,
            team,
            membership,
            ..authorized_context(&env, &user)
        },
    )
    .await
}
