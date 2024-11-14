use poem::{error::InternalServerError, session::Session, FromRequest, Request, RequestBody};
use serde::Serialize;

use crate::{
    app::errors::NotSignedInError,
    model::{types::Key, user::User},
};

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct SessionUser(pub User);

impl std::ops::Deref for SessionUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'r> FromRequest<'r> for SessionUser {
    async fn from_request(
        request: &'r Request,
        request_body: &mut RequestBody,
    ) -> poem::Result<Self> {
        let env = request
            .data::<crate::env::Env>()
            .expect("Env to be provided");

        let session = <&Session>::from_request(request, request_body).await?;

        let Some(user_id) = session.get::<Key<User>>("user_id") else {
            tracing::debug!("User not signed in (no 'user_id' in session)");
            session.set("destination", request.uri().to_string());
            session.set("error", "You need to sign in to access this resource");
            return Err(NotSignedInError.into());
        };

        let Some(user) = User::get(&env.pool, user_id)
            .await
            .map_err(InternalServerError)?
        else {
            tracing::error!("User {user_id} not found in database");
            session.remove("user_id");
            session.set("destination", request.uri().to_string());
            session.set("error", "You have been signed out");
            return Err(NotSignedInError.into());
        };

        if !user.enabled {
            tracing::error!("User {:?} ({user_id}) is disabled", user.username);
            session.remove("user_id");
            session.set("destination", request.uri().to_string());
            session.set("error", "You have been signed out");
            return Err(NotSignedInError.into());
        }

        // As the user is valid, we can set a 'last seen' variable in the session. This will
        // have the effect of updating the cookie we send to the user, which will keep the
        // session alive.
        session.set("last_seen", time::OffsetDateTime::now_utc().unix_timestamp());

        Ok(SessionUser(user))
    }
}
