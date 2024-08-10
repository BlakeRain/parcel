use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    session::Session,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Redirect},
};
use serde::Deserialize;

use crate::{
    app::{
        errors::CsrfError,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::user::User,
    utils::SessionExt,
};

#[handler]
pub fn get_settings(
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
    name: String,
}

#[handler]
pub async fn post_settings(
    env: Data<&Env>,
    mut user: User,
    verifier: &CsrfVerifier,
    session: &Session,
    Form(SettingsForm {
        token,
        username,
        name,
    }): Form<SettingsForm>,
) -> poem::Result<Redirect> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in settings form");
        return Err(CsrfError.into());
    }

    if user.username != username {
        if let Some(existing) =
            User::get_by_username(&env.pool, &username)
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
                name = user.name,
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
    }

    if user.name != name {
        tracing::info!(
            user_id = user.id,
            username = user.username,
            name = name,
            "Changing name"
        );

        user.set_name(&env.pool, &name).await.map_err(|err| {
            tracing::error!(user_id = user.id, username = ?username, err = ?err,
            "Failed to set name");
            InternalServerError(err)
        })?;
    }

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
