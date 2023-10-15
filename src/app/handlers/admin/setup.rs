use poem::{
    error::InternalServerError,
    handler,
    web::{Data, Form, Html},
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    app::templates::{default_context, render_template},
    env::Env,
    model::{hash_password, requires_setup, User},
};

#[handler]
pub async fn get_setup(env: Data<&Env>) -> poem::Result<Html<String>> {
    let required = requires_setup(&env.pool)
        .await
        .map_err(InternalServerError)?;

    let mut context = default_context();
    context.insert("required", &required);
    render_template("admin/setup.html", &context)
}

#[derive(Debug, Deserialize)]
pub struct SetupForm {
    username: String,
    password: String,
}

#[handler]
pub async fn post_setup(
    env: Data<&Env>,
    Form(SetupForm { username, password }): Form<SetupForm>,
) -> poem::Result<Html<String>> {
    let required = requires_setup(&env.pool)
        .await
        .map_err(InternalServerError)?;

    if !required {
        tracing::info!("Setup already completed");
        let mut context = default_context();
        context.insert("required", &false);
        return render_template("admin/setup.html", &context);
    }

    let mut admin = User {
        id: 0,
        username,
        password: hash_password(&password),
        enabled: true,
        admin: true,
        created_at: OffsetDateTime::now_utc(),
        created_by: None,
    };

    admin.create(&env.pool).await.map_err(InternalServerError)?;

    let mut context = default_context();
    context.insert("complete", &true);
    render_template("admin/setup.html", &context)
}
