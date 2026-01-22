use std::net::IpAddr;

use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    session::Session,
    web::{CsrfToken, CsrfVerifier, Data, Form, RealIp, Redirect, RemoteAddr},
    Addr, IntoResponse, Response,
};
use serde::Deserialize;

use parcel_model::{
    login_attempt::LoginAttempt,
    types::Key,
    user::{requires_setup, User},
};

use crate::{
    app::{
        errors::CsrfError,
        templates::{default_context, render_template},
    },
    env::Env,
    utils::SessionExt,
};

/// Get the client IP address, respecting the `trust_proxy` setting.
///
/// When `trust_proxy` is true, uses proxy headers (X-Forwarded-For, etc.).
/// When false, uses only the direct peer address to prevent IP spoofing.
fn get_client_ip(trust_proxy: bool, real_ip: &RealIp, remote_addr: &RemoteAddr) -> Option<IpAddr> {
    if trust_proxy {
        // RealIp already checks proxy headers and falls back to peer address
        real_ip.0
    } else {
        // Only use the direct peer address, ignore proxy headers
        match &remote_addr.0 {
            Addr::SocketAddr(addr) => Some(addr.ip()),
            _ => None,
        }
    }
}

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
    .await
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
    real_ip: RealIp,
    remote_addr: &RemoteAddr,
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

    let client_ip = get_client_ip(env.trust_proxy, &real_ip, remote_addr);
    let client_ip_str = client_ip.map(|ip| ip.to_string());

    // Check lockout BEFORE doing expensive password verification (prevents timing attacks)
    if LoginAttempt::is_locked_out(&env.pool, &username)
        .await
        .map_err(|err| {
            tracing::error!(?err, %username, "Failed to check lockout status");
            InternalServerError(err)
        })?
    {
        session.set(
            "error",
            "Too many failed attempts. Please try again in a few minutes.",
        );
        return Ok(Redirect::see_other("/user/signin"));
    }

    let user = User::get_by_username(&env.pool, &username)
        .await
        .map_err(|err| {
            tracing::error!(?username, ?err, "Failed to get user by username");
            InternalServerError(err)
        })?;

    let mut user = match user {
        Some(user) => user,
        None => {
            tracing::info!(?username, "User not found");
            // Record failed attempt even for non-existent users (prevents username enumeration timing)
            LoginAttempt::record(&env.pool, &username, client_ip_str.as_deref(), false)
                .await
                .ok();
            session.set("error", "Invalid username or password");
            return Ok(Redirect::see_other("/user/signin"));
        }
    };

    if !user.verify_password(&password) {
        tracing::info!(?username, "Invalid password");
        LoginAttempt::record(&env.pool, &username, client_ip_str.as_deref(), false)
            .await
            .ok();
        session.set("error", "Invalid username or password");
        return Ok(Redirect::see_other("/user/signin"));
    }

    if user.password.needs_migrating() {
        tracing::info!(%user.id, ?username, "Migrating password hash");
        user.set_password(&env.pool, &password).await?;
    }

    if !user.enabled {
        tracing::info!(?username, "User is disabled");
        session.set("error", "Your account is disabled");
        return Ok(Redirect::see_other("/user/signin"));
    }

    if user.totp.is_some() {
        tracing::info!(%user.id, ?username, "User requires TOTP");
        session.set("_authenticating", user.id);
        // Store username in session for TOTP lockout checks
        session.set("_authenticating_username", username);
        return Ok(Redirect::see_other("/user/signin/totp"));
    }

    // Record successful login
    LoginAttempt::record(&env.pool, &username, client_ip_str.as_deref(), true)
        .await
        .ok();

    session.remove("_authenticating");
    session.set("user_id", user.id);

    tracing::info!(%user.id, ?username, "User signed in");

    if let Some(destination) = session.take::<String>("destination") {
        Ok(Redirect::see_other(destination))
    } else {
        Ok(Redirect::see_other(if user.admin { "/admin" } else { "/" }))
    }
}

#[handler]
pub async fn get_signout(session: &Session) -> poem::Result<Redirect> {
    let mut stack = session
        .take::<Vec<Key<User>>>("masquerade_stack")
        .unwrap_or_default();
    if let Some(user_id) = stack.pop() {
        session.set("masquerade_stack", stack);
        session.set("user_id", user_id);
        return Ok(Redirect::see_other("/admin"));
    }

    session.clear();
    Ok(Redirect::see_other("/"))
}

#[handler]
pub async fn get_signin_totp(
    env: Data<&Env>,
    token: &CsrfToken,
    session: &Session,
) -> poem::Result<Response> {
    if session.get::<Key<User>>("_authenticating").is_none() {
        tracing::error!("User not authenticating");
        session.set("error", "You need to sign in first");
        return Ok(Redirect::see_other("/user/signin").into_response());
    }

    Ok(render_template(
        "user/totp.html",
        context! {
            token => token.0,
            error => session.take::<String>("error"),
            ..default_context(&env)
        },
    )
    .await?
    .into_response())
}

#[derive(Debug, Deserialize)]
pub struct TotpForm {
    token: String,
    code: String,
}

#[handler]
pub async fn post_signin_totp(
    env: Data<&Env>,
    session: &Session,
    verifier: &CsrfVerifier,
    real_ip: RealIp,
    remote_addr: &RemoteAddr,
    Form(TotpForm { token, code }): Form<TotpForm>,
) -> poem::Result<Redirect> {
    let Some(user_id) = session.get::<Key<User>>("_authenticating") else {
        tracing::error!("User not authenticating");
        session.set("error", "You need to sign in first");
        return Ok(Redirect::see_other("/user/signin"));
    };

    // Get the username from session for lockout checks (shared counter with password)
    let username = session
        .get::<String>("_authenticating_username")
        .unwrap_or_default();

    let client_ip = get_client_ip(env.trust_proxy, &real_ip, remote_addr);
    let client_ip_str = client_ip.map(|ip| ip.to_string());

    // Check lockout (shared counter with password attempts)
    if !username.is_empty()
        && LoginAttempt::is_locked_out(&env.pool, &username)
            .await
            .map_err(|err| {
                tracing::error!(?err, %username, "Failed to check lockout status");
                InternalServerError(err)
            })?
    {
        session.remove("_authenticating");
        session.remove("_authenticating_username");
        session.set(
            "error",
            "Too many failed attempts. Please try again in a few minutes.",
        );
        return Ok(Redirect::see_other("/user/signin"));
    }

    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in sign in TOTP form");
        return Err(CsrfError.into());
    }

    let Some(user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(%user_id, ?err, "Failed to get user by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%user_id, "User not found");
        session.remove("_authenticating");
        session.remove("_authenticating_username");
        session.set("error", "You need to sign in first");
        return Ok(Redirect::see_other("/user/signin"));
    };

    let Some(ref secret) = user.totp else {
        tracing::error!(%user_id, "User does not have TOTP secret");
        session.remove("_authenticating");
        session.remove("_authenticating_username");
        session.set("error", "You need to sign in first");
        return Ok(Redirect::see_other("/user/signin"));
    };

    let totp = code.trim();
    if !totp.chars().all(|c| c.is_ascii_digit()) {
        tracing::error!(
            %user.id,
            "TOTP code provided was not a sequence of ASCII digits"
        );

        session.set(
            "totp_error",
            "🤨 Your TOTP code should be a sequence of six numbers. Try again.",
        );

        return Ok(Redirect::see_other("/user/signin/totp"));
    }

    if totp.len() != 6 {
        tracing::error!(
            %user.id,
            length = totp.len(),
            "Incorrect number of digits provided for TOTP code (expected 6)"
        );

        session.set(
            "error",
            "🤨 Your TOTP code should be a sequence of six numbers. Try again.",
        );

        return Ok(Redirect::see_other("/user/signin/totp"));
    }

    let Some(decoded_secret) = base32::decode(base32::Alphabet::Rfc4648 { padding: true }, secret)
    else {
        tracing::error!(
            %user.id,
            "TOTP secret from session was not valid base-32"
        );

        session.set(
            "error",
            "😒 There was a problem with the MFA setup process. Please try again.",
        );

        return Ok(Redirect::see_other("/user/signin/totp"));
    };

    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let expected = totp_lite::totp_custom::<totp_lite::Sha1>(
        totp_lite::DEFAULT_STEP,
        6,
        &decoded_secret[..],
        seconds,
    );

    if totp != expected {
        tracing::error!(
            %user.id,
            "TOTP code provided did not match the expected value"
        );

        // Record failed TOTP attempt (shared counter with password)
        if !username.is_empty() {
            LoginAttempt::record(&env.pool, &username, client_ip_str.as_deref(), false)
                .await
                .ok();
        }

        session.set(
            "error",
            "🤨 The TOTP code you provided was incorrect. Please try again.",
        );

        return Ok(Redirect::see_other("/user/signin/totp"));
    }

    // Record successful login
    if !username.is_empty() {
        LoginAttempt::record(&env.pool, &username, client_ip_str.as_deref(), true)
            .await
            .ok();
    }

    session.remove("_authenticating");
    session.remove("_authenticating_username");
    session.set("user_id", user.id);

    tracing::info!(%user.id, "User signed in after TOTP");

    if let Some(destination) = session.take::<String>("destination") {
        Ok(Redirect::see_other(destination))
    } else {
        Ok(Redirect::see_other(if user.admin { "/admin" } else { "/" }))
    }
}
