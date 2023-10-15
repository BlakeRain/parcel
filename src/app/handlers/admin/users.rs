use poem::{
    error::InternalServerError,
    handler,
    web::{Data, Form, Html, Path, Redirect},
    IntoResponse,
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    app::templates::{default_context, render_template},
    env::Env,
    model::{hash_password, User},
};

#[handler]
pub async fn get_users(env: Data<&Env>) -> poem::Result<Html<String>> {
    let users = User::get_list(&env.pool)
        .await
        .map_err(InternalServerError)?;
    let mut context = default_context();
    context.insert("users", &users);
    render_template("admin/users.html", &context)
}

#[derive(Debug, Deserialize)]
pub struct NewUserForm {
    username: String,
    password: String,
    admin: bool,
}

#[handler]
pub async fn post_users(
    env: Data<&Env>,
    Form(NewUserForm {
        username,
        password,
        admin,
    }): Form<NewUserForm>,
) -> poem::Result<impl IntoResponse> {
    let mut user = User {
        id: 0,
        username,
        password: hash_password(&password),
        enabled: true,
        admin,
        created_at: OffsetDateTime::now_utc(),
        created_by: None,
    };

    user.create(&env.pool).await.map_err(InternalServerError)?;
    Ok(Redirect::see_other("/admin/users"))
}

#[derive(Debug, Deserialize)]
pub struct EditUserForm {
    username: Option<String>,
    password: Option<String>,
    admin: bool,
    enabled: bool,
}

#[handler]
pub async fn post_user(
    env: Data<&Env>,
    Path(user_id): Path<i32>,
    Form(EditUserForm {
        username,
        password,
        admin,
        enabled,
    }): Form<EditUserForm>,
) -> poem::Result<impl IntoResponse> {
    let Some(mut user) = User::get(&env.pool, user_id)
        .await
        .map_err(InternalServerError)?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return Ok(Redirect::see_other("/admin/users"));
    };

    if let Some(username) = username {
        if username != user.username {
            let existing = User::get_by_username(&env.pool, &username)
                .await
                .map_err(InternalServerError)?;

            if existing.is_some() {
                tracing::error!("Username '{username}' already exists");
                return Ok(Redirect::see_other("/admin/users"));
            }

            user.username = username;
        }
    }

    Ok(Redirect::see_other("/admin/users"))
}
