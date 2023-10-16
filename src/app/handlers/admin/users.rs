use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path, Redirect},
    IntoResponse,
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    app::templates::{default_context, render_404, render_template},
    env::Env,
    model::user::{hash_password, User},
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

#[handler]
pub fn get_users_new(env: Data<&Env>, token: &CsrfToken) -> poem::Result<Html<String>> {
    let mut context = default_context();
    context.insert("token", &token.0);
    render_template("admin/users/new.html", &context)
}

#[derive(Debug, Deserialize)]
pub struct NewUserForm {
    token: String,
    username: String,
    password: String,
    admin: bool,
}

#[handler]
pub async fn post_users_new(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    Form(NewUserForm {
        token,
        username,
        password,
        admin,
    }): Form<NewUserForm>,
) -> poem::Result<impl IntoResponse> {
    if !verifier.is_valid(&token) {
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

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
pub async fn get_user_edit(
    env: Data<&Env>,
    Path(user_id): Path<i32>,
) -> poem::Result<Html<String>> {
    let Some(user) = User::get(&env.pool, user_id)
        .await
        .map_err(InternalServerError)?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return render_404("Unrecognized user ID");
    };

    let mut context = default_context();
    context.insert("user", &user);
    render_template("admin/users/edit.html", &context)
}

#[handler]
pub async fn put_user(
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
