use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    session::Session,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Redirect},
    IntoResponse, Response,
};
use serde::Deserialize;

use crate::{
    app::{
        errors::CsrfError,
        templates::{authorized_context, default_context, render_template},
    },
    env::Env,
    model::user::{requires_setup, User},
    utils::SessionExt,
};

#[handler]
pub async fn get_signin(
    env: Data<&Env>,
    token: &CsrfToken,
    session: &Session,
) -> poem::Result<Response> {
    let setup = requires_setup(&env.pool).await.map_err(|err| {
        tracing::error!(error = ?err, "Failed to check if setup is required");
        InternalServerError(err)
    })?;

    if setup {
        return Ok(Redirect::see_other("/admin/setup").into_response());
    }

    render_template(
        "user/signin.html",
        context! {
            token => token.0,
            error => session.take::<String>("error"),
            ..default_context(&env)
        },
    )
    .map(IntoResponse::into_response)
}

#[derive(Debug, Deserialize)]
pub struct SignInForm {
    token: String,
    username: String,
    password: String,
}

#[handler]
pub async fn post_signin(
    env: Data<&Env>,
    session: &Session,
    verifier: &CsrfVerifier,
    Form(SignInForm {
        token,
        username,
        password,
    }): Form<SignInForm>,
) -> poem::Result<Redirect> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in sign in form");
        return Err(CsrfError.into());
    }

    let user = User::get_by_username(&env.pool, &username)
        .await
        .map_err(|err| {
            tracing::error!(username = ?username, err = ?err,
                            "Failed to get user by username");
            InternalServerError(err)
        })?;

    let user = match user {
        Some(user) => user,
        None => {
            tracing::info!(username = ?username, "User not found");
            session.set("error", "Invalid username or password");
            return Ok(Redirect::see_other("/user/signin"));
        }
    };

    if !user.verify_password(&password) {
        tracing::info!(username = ?username, "Invalid password");
        session.set("error", "Invalid username or password");
        return Ok(Redirect::see_other("/user/signin"));
    }

    if !user.enabled {
        tracing::info!(username = ?username, "User is disabled");
        session.set("error", "Your account is disabled");
        return Ok(Redirect::see_other("/user/signin"));
    }

    session.set("user_id", user.id);

    tracing::info!(user_id = user.id, username = ?username, "User signed in");

    if let Some(destination) = session.take::<String>("destination") {
        Ok(Redirect::see_other(destination))
    } else {
        Ok(Redirect::see_other(if user.admin { "/admin" } else { "/" }))
    }
}

#[handler]
pub async fn get_signout(session: &Session) -> poem::Result<Redirect> {
    session.clear();
    Ok(Redirect::see_other("/"))
}

#[handler]
pub async fn get_settings(
    env: Data<&Env>,
    user: User,
    session: &Session,
    token: &CsrfToken,
) -> poem::Result<Html<String>> {
    render_template(
        "user/settings.html",
        context! {
            token => token.0,
            settings_error => session.take::<String>("settings_error"),
            settings_success => session.take::<String>("settings_success"),
            password_error => session.take::<String>("password_error"),
            password_success => session.take::<String>("password_success"),
            ..authorized_context(&env, &user)
        },
    )
}

#[derive(Debug, Deserialize)]
pub struct SettingsForm {
    token: String,
    username: String,
}

#[handler]
pub async fn post_settings(
    env: Data<&Env>,
    mut user: User,
    verifier: &CsrfVerifier,
    session: &Session,
    Form(SettingsForm { token, username }): Form<SettingsForm>,
) -> poem::Result<Redirect> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in settings form");
        return Err(CsrfError.into());
    }

    if user.username == username {
        tracing::info!("Username was not changed; ignoring settings change");
        return Ok(Redirect::see_other("/user/settings"));
    }

    if let Some(existing) = User::get_by_username(&env.pool, &username)
        .await
        .map_err(|err| {
            tracing::error!(username = ?username, err = ?err,
                    "Failed to get user by username");
            InternalServerError(err)
        })?
    {
        tracing::error!(
            user_id = user.id,
            username = user.username,
            new_username = username,
            existing_id = existing.id,
            "Username is already taken"
        );

        session.set("settings_error", "Username is already taken");
        return Ok(Redirect::see_other("/user/settings"));
    }

    tracing::info!(
        user_id = user.id,
        username = user.username,
        new_username = username,
        "Changing username"
    );

    user.set_username(&env.pool, &username)
        .await
        .map_err(|err| {
            tracing::error!(user_id = user.id, username = ?username, err = ?err,
                    "Failed to set username");
            InternalServerError(err)
        })?;

    session.set(
        "settings_success",
        "Your account settings have been updated successfully",
    );

    Ok(Redirect::see_other("/user/settings"))
}

#[derive(Debug, Deserialize)]
pub struct PasswordForm {
    token: String,
    password: String,
}

#[handler]
pub async fn post_password(
    env: Data<&Env>,
    mut user: User,
    verifier: &CsrfVerifier,
    session: &Session,
    Form(PasswordForm { token, password }): Form<PasswordForm>,
) -> poem::Result<Redirect> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in password form");
        return Err(CsrfError.into());
    }

    tracing::info!(
        user_id = user.id,
        username = user.username,
        "Changing password"
    );

    user.set_password(&env.pool, &password)
        .await
        .map_err(|err| {
            tracing::error!(user_id = user.id, username = ?user.username, err = ?err,
                    "Failed to set password");
            InternalServerError(err)
        })?;

    session.set(
        "password_success",
        "Your password has been updated successfully",
    );

    Ok(Redirect::see_other("/user/settings"))
}
