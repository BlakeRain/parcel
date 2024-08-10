use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    session::Session,
    web::{CsrfToken, CsrfVerifier, Data, Form, Redirect},
    IntoResponse, Response,
};
use serde::Deserialize;

use crate::{
    app::{
        errors::CsrfError,
        templates::{default_context, render_template},
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
