use parcel_model::user::User;
use parcel_shared::types::api::ApiMeResponse;
use poem::web::{Data, Json};

use crate::{app::extractors::api_key::BearerApiKey, env::Env};

mod teams;
mod uploads;

pub use teams::{get_team, get_teams};
pub use uploads::{get_uploads, get_team_uploads, get_upload, put_upload, post_upload, delete_upload};

#[poem::handler]
pub async fn get_me(env: Data<&Env>, api_key: BearerApiKey) -> poem::Result<Json<ApiMeResponse>> {
    tracing::info!(key = ?api_key.name, "API key used to get user info");

    let Some(user) = User::get(&env.pool, api_key.owner).await.map_err(|err| {
        tracing::error!(api_key = ?api_key.name, ?err, "Failed to get user by API key owner");
        poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
    })?
    else {
        tracing::error!(api_key = ?api_key.name, "User not found for API key owner");
        return Err(poem::Error::from_string(
            "User not found",
            poem::http::StatusCode::NOT_FOUND,
        ));
    };

    let response = ApiMeResponse {
        id: user.id.into(),
        username: user.username,
        name: user.name,
        last_access: user.last_access,
    };

    Ok(Json(response))
}
