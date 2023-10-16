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
    app::{
        extractors::admin::Admin,
        templates::{authorized_context, render_404, render_template},
    },
    env::Env,
    model::user::{hash_password, User},
};

#[handler]
pub async fn get_users(env: Data<&Env>, Admin(admin): Admin) -> poem::Result<Html<String>> {
    let users = User::get_list(&env.pool)
        .await
        .map_err(InternalServerError)?;
    let mut context = authorized_context(&admin);
    context.insert("users", &users);
    render_template("admin/users.html", &context)
}

#[handler]
pub fn get_users_new(Admin(admin): Admin, token: &CsrfToken) -> poem::Result<Html<String>> {
    let mut context = authorized_context(&admin);
    context.insert("token", &token.0);
    render_template("admin/users/new.html", &context)
}

#[derive(Debug, Deserialize)]
pub struct NewUserForm {
    token: String,
    username: String,
    password: String,
    admin: Option<String>,
    enabled: Option<String>,
}

#[handler]
pub async fn post_users_new(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    Admin(admin_user): Admin,
    Form(NewUserForm {
        token,
        username,
        password,
        admin,
        enabled,
    }): Form<NewUserForm>,
) -> poem::Result<impl IntoResponse> {
    if !verifier.is_valid(&token) {
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let admin = admin.as_deref() == Some("on");
    let enabled = enabled.as_deref() == Some("on");

    let mut user = User {
        id: 0,
        username,
        password: hash_password(&password),
        enabled,
        admin,
        created_at: OffsetDateTime::now_utc(),
        created_by: Some(admin_user.id),
    };

    user.create(&env.pool).await.map_err(InternalServerError)?;
    Ok(Redirect::see_other("/admin"))
}

#[handler]
pub async fn get_user_edit(
    env: Data<&Env>,
    token: &CsrfToken,
    Path(user_id): Path<i32>,
    Admin(admin): Admin,
) -> poem::Result<Html<String>> {
    let Some(user) = User::get(&env.pool, user_id)
        .await
        .map_err(InternalServerError)?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return render_404("Unrecognized user ID");
    };

    let mut context = authorized_context(&admin);
    context.insert("token", &token.0);
    context.insert("user", &user);
    render_template("admin/users/edit.html", &context)
}

#[derive(Debug, Deserialize)]
pub struct EditUserForm {
    token: String,
    username: String,
    admin: Option<String>,
    enabled: Option<String>,
}

#[handler]
pub async fn put_user(
    env: Data<&Env>,
    Admin(_): Admin,
    Path(user_id): Path<i32>,
    verifier: &CsrfVerifier,
    Form(EditUserForm {
        token,
        username,
        admin,
        enabled,
    }): Form<EditUserForm>,
) -> poem::Result<impl IntoResponse> {
    if !verifier.is_valid(&token) {
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(mut user) = User::get(&env.pool, user_id)
        .await
        .map_err(InternalServerError)?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return Ok(Redirect::see_other("/admin/users"));
    };

    if username != user.username {
        let existing = User::get_by_username(&env.pool, &username)
            .await
            .map_err(InternalServerError)?;

        if existing.is_some() {
            tracing::error!("Username '{username}' already exists");
            return Ok(Redirect::see_other("/admin/users"));
        }
    }

    let admin = admin.as_deref() == Some("on");
    let enabled = enabled.as_deref() == Some("on");

    user.update(&env.pool, &username, admin, enabled)
        .await
        .map_err(InternalServerError)?;

    Ok(Redirect::see_other("/admin/users"))
}

#[handler]
pub async fn delete_user(
    env: Data<&Env>,
    Admin(_): Admin,
    Path(user_id): Path<i32>,
) -> poem::Result<Redirect> {
    User::delete(&env.pool, user_id)
        .await
        .map_err(InternalServerError)?;
    Ok(Redirect::see_other("/admin/users"))
}

#[handler]
pub async fn put_disable_user(
    env: Data<&Env>,
    Path(user_id): Path<i32>,
    Admin(_): Admin,
) -> poem::Result<Redirect> {
    let Some(mut user) = User::get(&env.pool, user_id)
        .await
        .map_err(InternalServerError)?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    user.set_enabled(&env.pool, false)
        .await
        .map_err(InternalServerError)?;

    Ok(Redirect::see_other("/admin/users"))
}

#[handler]
pub async fn put_enable_user(
    env: Data<&Env>,
    Path(user_id): Path<i32>,
    Admin(_): Admin,
) -> poem::Result<Redirect> {
    let Some(mut user) = User::get(&env.pool, user_id)
        .await
        .map_err(InternalServerError)?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    user.set_enabled(&env.pool, true)
        .await
        .map_err(InternalServerError)?;

    Ok(Redirect::see_other("/admin/users"))
}
