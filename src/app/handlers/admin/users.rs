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
    let users = User::get_list(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, "Failed to get list of users");
        InternalServerError(err)
    })?;

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
    limit: Option<i32>,
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
        limit,
    }): Form<NewUserForm>,
) -> poem::Result<impl IntoResponse> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in new user form");
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
        limit: limit.map(|limit| limit * 1024 * 1024),
        created_at: OffsetDateTime::now_utc(),
        created_by: Some(admin_user.id),
    };

    user.create(&env.pool).await.map_err(|err| {
        tracing::error!(user = ?user, err = ?err, "Failed to create new user");
        InternalServerError(err)
    })?;

    tracing::info!(user = ?user.id, username = ?user.username, "Created new user");

    Ok(Redirect::see_other("/admin/users"))
}

#[handler]
pub async fn get_user_edit(
    env: Data<&Env>,
    token: &CsrfToken,
    Path(user_id): Path<i32>,
    Admin(admin): Admin,
) -> poem::Result<Html<String>> {
    let Some(user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!(user_id = user_id, "Unrecognized user ID");
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
    limit: Option<i32>,
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
        limit,
    }): Form<EditUserForm>,
) -> poem::Result<impl IntoResponse> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in edit user form");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(mut user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!(user_id = user_id, "Unrecognized user ID");
        return Ok(Redirect::see_other("/admin/users"));
    };

    if username != user.username {
        let existing = User::get_by_username(&env.pool, &username)
            .await
            .map_err(|err| {
                tracing::error!(err = ?err, username = ?username, 
                                "Failed to query for user with existing username");
                InternalServerError(err)
            })?;

        if existing.is_some() {
            tracing::error!(usernae = ?username, "Username already exists");
            return Ok(Redirect::see_other("/admin/users"));
        }
    }

    let admin = admin.as_deref() == Some("on");
    let enabled = enabled.as_deref() == Some("on");
    let limit = limit.map(|limit| limit * 1024 * 1024);

    user.update(&env.pool, &username, admin, enabled, limit)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, user_id = user_id, "Failed to update user");
            InternalServerError(err)
        })?;

    tracing::info!(
        user_id = user_id, admin = admin, enabled = enabled, limit = ?limit,
        "Updated user"
    );

    Ok(Redirect::see_other("/admin/users"))
}

#[handler]
pub async fn delete_user(
    env: Data<&Env>,
    Admin(_): Admin,
    Path(user_id): Path<i32>,
) -> poem::Result<Redirect> {
    User::delete(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = user_id, "Failed to delete user");
        InternalServerError(err)
    })?;

    tracing::info!(user_id = user_id, "Deleted user");
    Ok(Redirect::see_other("/admin/users"))
}

#[handler]
pub async fn put_disable_user(
    env: Data<&Env>,
    Path(user_id): Path<i32>,
    Admin(_): Admin,
) -> poem::Result<Redirect> {
    let Some(mut user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    user.set_enabled(&env.pool, false).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = user_id, "Failed to disable user");
        InternalServerError(err)
    })?;

    tracing::info!(user_id = user_id, "Disabled user");
    Ok(Redirect::see_other("/admin/users"))
}

#[handler]
pub async fn put_enable_user(
    env: Data<&Env>,
    Path(user_id): Path<i32>,
    Admin(_): Admin,
) -> poem::Result<Redirect> {
    let Some(mut user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    user.set_enabled(&env.pool, true).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = user_id, "Failed to enable user");
        InternalServerError(err)
    })?;

    tracing::info!(user_id = user_id, "Enabled user");
    Ok(Redirect::see_other("/admin/users"))
}
