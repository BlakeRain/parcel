use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    web::{Data, Html},
};

use crate::{
    app::{
        extractors::admin::Admin,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{upload::UploadStats, user::UserStats},
};

pub mod setup;
pub mod uploads;
pub mod users;

#[handler]
pub async fn get_admin(env: Data<&Env>, Admin(admin): Admin) -> poem::Result<Html<String>> {
    let users = UserStats::get(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, "Failed to get user stats for admin dashboard");
        InternalServerError(err)
    })?;

    let uploads = UploadStats::get(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, "Failed to get upload stats for admin dashboard");
        InternalServerError(err)
    })?;

    render_template(
        "admin/index.html",
        context! {
            users,
            uploads,
            ..authorized_context(&env, &admin)
        },
    )
}
