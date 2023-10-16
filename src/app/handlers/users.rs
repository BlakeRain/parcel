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
        templates::{authorized_context, default_context, render_template},
    },
    env::Env,
    model::user::User,
};

#[handler]
pub fn get_signin(token: &CsrfToken, session: &Session) -> poem::Result<Html<String>> {
    let error = session.get::<String>("error");
    session.remove("error");

    let mut context = default_context();
    context.insert("token", &token.0);
    context.insert("error", &error);
    render_template("user/signin.html", &context)
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
        return Err(CsrfError.into());
    }

    let user = match User::get_by_username(&env.pool, &username)
        .await
        .map_err(InternalServerError)?
    {
        Some(user) => user,
        None => {
            session.set("error", "Invalid username or password");
            return Ok(Redirect::see_other("/user/signin"));
        }
    };

    if !user.verify_password(&password) {
        session.set("error", "Invalid username or password");
        return Ok(Redirect::see_other("/user/signin"));
    }

    session.set("user_id", user.id);

    if let Some(destination) = session.get::<String>("destination") {
        session.remove("destination");
        Ok(Redirect::see_other(destination))
    } else {
        Ok(Redirect::see_other(if user.admin { "/admin" } else { "/" }))
    }
}

#[handler]
pub async fn get_signout(session: &Session) -> poem::Result<Redirect> {
    session.remove("user_id");
    Ok(Redirect::see_other("/"))
}

#[handler]
pub async fn get_settings(user: User, token: &CsrfToken) -> poem::Result<Html<String>> {
    let mut context = authorized_context(&user);
    context.insert("token", &token.0);
    render_template("user/settings.html", &context)
}
