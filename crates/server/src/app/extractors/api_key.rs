use parcel_model::{api_key::ApiKey, user::User};
use poem::{
    http::StatusCode,
    web::headers::{authorization::Bearer, Authorization, HeaderMapExt},
    FromRequest, Request, RequestBody,
};

pub struct BearerApiKey {
    pub key: ApiKey,
    pub user: User,
}

impl std::ops::Deref for BearerApiKey {
    type Target = ApiKey;

    fn deref(&self) -> &Self::Target {
        &self.key
    }
}

impl<'r> FromRequest<'r> for BearerApiKey {
    async fn from_request(request: &'r Request, _: &mut RequestBody) -> poem::Result<Self> {
        let env = request
            .data::<crate::env::Env>()
            .expect("Env to be provided");

        let Some(authorization) = request.headers().typed_get::<Authorization<Bearer>>() else {
            tracing::error!("No Authorization header found");
            return Err(poem::Error::from_string(
                "Authorization header not found",
                StatusCode::UNAUTHORIZED,
            ));
        };

        let Some(mut key) = ApiKey::get_by_code(&env.pool, authorization.token())
            .await
            .map_err(|err| {
                tracing::error!(?err, "Failed to get API key by code");
                poem::Error::from_string("Invalid API key", StatusCode::FORBIDDEN)
            })?
        else {
            tracing::error!("Invalid API key in Authorization header");
            return Err(poem::Error::from_string(
                "Invalid API key",
                StatusCode::UNAUTHORIZED,
            ));
        };

        if !key.enabled {
            tracing::error!(key = ?key.name, "API key is disabled");
            return Err(poem::Error::from_string(
                "Invalid API key",
                StatusCode::FORBIDDEN,
            ));
        }

        let user = User::get(&env.pool, key.owner)
            .await
            .map_err(|err| {
                tracing::error!(key = ?key.name, ?err, "Failed to get user by API key owner");
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?
            .ok_or_else(|| {
                tracing::error!(key = ?key.name, "User not found for API key owner");
                poem::Error::from_string("User not found", StatusCode::NOT_FOUND)
            })?;

        if !user.enabled {
            tracing::error!(key = ?key.name, user = %user.id, "User is disabled");
            return Err(poem::Error::from_string(
                "User is disabled",
                StatusCode::FORBIDDEN,
            ));
        }

        key.record_last_use(&env.pool).await.map_err(|err| {
            tracing::error!(key = ?key.name, ?err, "Failed to record last use of API key");
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Self { key, user })
    }
}
