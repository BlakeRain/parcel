use poem::{http::StatusCode, FromRequest, Request, RequestBody};

use crate::model::user::User;

pub struct Admin(pub User);

impl std::ops::Deref for Admin {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'r> FromRequest<'r> for Admin {
    async fn from_request(
        request: &'r Request,
        request_body: &mut RequestBody,
    ) -> poem::Result<Self> {
        let user = User::from_request(request, request_body).await?;
        if user.admin {
            Ok(Admin(user))
        } else {
            tracing::warn!(
                "Non-admin user {:?} ({}) attempted to access admin-only page",
                user.username,
                user.id
            );

            Err(poem::Error::from_status(StatusCode::FORBIDDEN))
        }
    }
}
