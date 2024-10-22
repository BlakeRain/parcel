use minijinja::context;
use poem::{
    error::InternalServerError,
    http::StatusCode,
    web::{Data, Html, Path, Query},
};

use crate::{
    app::{
        extractors::user::SessionUser,
        handlers::uploads::ListQuery,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{team::Team, types::Key, upload::Upload},
};

#[poem::handler]
pub async fn get_list(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(team_id): Path<Key<Team>>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    // See if we can get the team with the given ID
    let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(%team_id, ?err, "Unable to get team");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%team_id, "Team not found");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    // Check to make sure that this user is a member of the team.
    let is_member = user.is_member_of(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(%user.id, %team_id, ?err, "Unable to check if user is a member of team");
        InternalServerError(err)
    })?;

    if !is_member {
        tracing::error!(%user.id, %team_id, "User is not a member of team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    let total = Upload::count_for_team(&env.pool, team_id)
        .await
        .map_err(|err| {
            tracing::error!(%team_id, ?err, "Unable to get upload count for team");
            InternalServerError(err)
        })?;

    let uploads = Upload::get_for_team(&env.pool, team.id, query.order, query.asc, 0, 50)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get uploads for team");
            InternalServerError(err)
        })?;

    render_template(
        "uploads/list.html",
        context! {
            total,
            uploads,
            query,
            ..authorized_context(&env, &user)
        },
    )
}

#[poem::handler]
pub async fn get_page(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path((team_id, page)): Path<(Key<Team>, u32)>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    // See if we can find the team with the given ID.
    let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(%team_id, ?err, "Unable to get team");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%team_id, "Team not found");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    // Check to make sure that this user is a member of the team.
    let is_member = user.is_member_of(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(%user.id, %team_id, ?err, "Unable to check if user is a member of team");
        InternalServerError(err)
    })?;

    if !is_member {
        tracing::error!(%user.id, %team_id, "User is not a member of team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    let total = Upload::count_for_team(&env.pool, team_id)
        .await
        .map_err(|err| {
            tracing::error!(%team_id, ?err, "Unable to get upload count for team");
            InternalServerError(err)
        })?;

    let last_page = total / 50;
    let page = page.min(last_page);
    let offset = page * 50;
    let uploads = Upload::get_for_team(&env.pool, team.id, query.order, query.asc, offset, 50)
        .await
        .map_err(|err| {
            tracing::error!(%team.id, ?err, "Unable to get uploads for team");
            InternalServerError(err)
        })?;

    render_template(
        "uploads/page.html",
        context! {
            page,
            last_page,
            uploads,
            query,
            team,
            ..authorized_context(&env, &user)
        },
    )
}
