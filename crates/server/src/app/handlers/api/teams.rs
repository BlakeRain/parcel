use parcel_model::{team::Team, types::Key};
use parcel_shared::types::api::{ApiTeamInfo, ApiTeamResponse, ApiTeamsResponse};
use poem::{
    http::StatusCode,
    web::{Data, Json, Path},
};

use crate::{app::extractors::api_key::BearerApiKey, env::Env};

#[poem::handler]
pub async fn get_teams(
    env: Data<&Env>,
    api_key: BearerApiKey,
) -> poem::Result<Json<ApiTeamsResponse>> {
    tracing::info!(key = ?api_key.name, "API key used to get teams");

    let teams = Team::get_for_user(&env.pool, api_key.owner)
        .await
        .map_err(|err| {
            tracing::error!(api_key = ?api_key.name, ?err, "Failed to get teams for API key owner");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    let teams = teams
        .into_iter()
        .map(|team| ApiTeamInfo {
            id: team.id.into(),
            name: team.name,
            slug: team.slug,
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiTeamsResponse { teams }))
}

#[poem::handler]
pub async fn get_team(
    env: Data<&Env>,
    api_key: BearerApiKey,
    Path(team_id): Path<Key<Team>>,
) -> poem::Result<Json<ApiTeamResponse>> {
    tracing::info!(key = ?api_key.name, %team_id, "API key used to get team info");

    let is_member = api_key
        .user
        .is_member_of(&env.pool, team_id)
        .await
        .map_err(|err| {
            tracing::error!(api_key = ?api_key.name, ?err, "Failed to check team membership");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    if !is_member {
        tracing::error!(
            api_key = ?api_key.name,
            owner = ?api_key.user.username,
            %team_id,
            "API key owner is not a member of the team"
        );

        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    }

    let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(api_key = ?api_key.name, ?err, "Failed to get team by ID");
        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?
    else {
        tracing::error!(api_key = ?api_key.name, %team_id, "Team not found");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let team = ApiTeamInfo {
        id: team.id.into(),
        name: team.name,
        slug: team.slug,
    };

    Ok(Json(ApiTeamResponse { team }))
}
