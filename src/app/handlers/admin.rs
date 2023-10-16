use poem::{
    handler,
    web::{Data, Html},
};

use crate::{
    app::{
        extractors::admin::Admin,
        templates::{authorized_context, default_context, render_template},
    },
    env::Env,
};

pub mod setup;
pub mod users;

#[handler]
pub fn get_admin(env: Data<&Env>, Admin(admin): Admin) -> poem::Result<Html<String>> {
    tracing::info!("Admin: {:?}", admin);
    let context = authorized_context(&admin);
    render_template("admin.html", &context)
}
