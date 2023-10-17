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
    let users = UserStats::get(&env.pool)
        .await
        .map_err(InternalServerError)?;

    let uploads = UploadStats::get(&env.pool)
        .await
        .map_err(InternalServerError)?;

    let mut context = authorized_context(&admin);
    context.insert("users", &users);
    context.insert("uploads", &uploads);
    render_template("admin/index.html", &context)
}
