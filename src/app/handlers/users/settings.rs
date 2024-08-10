use fast_qr::{
    convert::{svg::SvgBuilder, Builder, Shape},
    QRBuilder,
};
use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    session::Session,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Redirect},
    IntoResponse, Response,
};
use rand::Rng;
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

const TOTP_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const TOTP_SECRET_LEN: usize = 32;

fn generate_totp_secret() -> String {
    let mut rng = rand::thread_rng();
    (0..TOTP_SECRET_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..TOTP_CHARSET.len());
            TOTP_CHARSET[idx] as char
        })
        .collect()
}

#[handler]
pub fn get_setup_totp(
    env: Data<&Env>,
    user: User,
    csrf_token: &CsrfToken,
    session: &Session,
) -> poem::Result<Html<String>> {
    let secret = match session.get::<String>("totp_secret") {
        Some(secret) => secret,
        None => {
            let secret = generate_totp_secret();
            session.set("totp_secret", &secret);
            secret
        }
    };

    let otp_url = format!(
        "otpauth://totp/Parcel:{}?secret={secret}&issuer=Parcel&algorithm=SHA1&digits=6&period=30",
        user.username
    );

    let qrcode = QRBuilder::new(otp_url.as_bytes()).build().map_err(|err| {
        tracing::error!(err = ?err, "Failed to generate QR code");
        InternalServerError(err)
    })?;

    let svg = SvgBuilder::default().shape(Shape::Square).to_str(&qrcode);

    render_template(
        "user/setup-totp.html",
        context! {
            secret,
            otp_url,
            svg,
            csrf_token => csrf_token.0,
            totp_error => session.take::<String>("totp_error"),
            ..authorized_context(&env, &user)
        },
    )
}

#[derive(Debug, Deserialize)]
pub struct SetupTotpForm {
    csrf_token: String,
    code: String,
}

#[handler]
pub async fn post_setup_totp(
    env: Data<&Env>,
    mut user: User,
    verifier: &CsrfVerifier,
    session: &Session,
    Form(form): Form<SetupTotpForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&form.csrf_token) {
        tracing::error!("invalid CSRF token in setup TOTP form");
        return Err(CsrfError.into());
    }

    let totp = form.code.trim();
    if !totp.chars().all(|c| c.is_ascii_digit()) {
        tracing::error!(
            user = user.id,
            "TOTP code provided was not a sequence of ASCII digits"
        );

        session.set(
            "totp_error",
            "🤨 Your TOTP code should be a sequence of six numbers. Try again.",
        );

        return Ok(Redirect::see_other("/user/settings/totp").into_response());
    }

    if totp.len() != 6 {
        tracing::error!(
            user = user.id,
            length = totp.len(),
            "Incorrect number of digits provided for TOTP code (expected 6)"
        );

        session.set(
            "totp_error",
            "🤨 Your TOTP code should be a sequence of six numbers. Try again.",
        );

        return Ok(Redirect::see_other("/user/settings/totp").into_response());
    }

    let Some(secret) = session.get::<String>("totp_secret") else {
        tracing::error!(user = user.id, "TOTP secret missing from session");

        session.set(
            "totp_error",
            "😒 There was a problem with the MFA setup process. Please try again.",
        );

        return Ok(Redirect::see_other("/user/settings/totp").into_response());
    };

    let Some(decoded_secret) = base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret)
    else {
        tracing::error!(
            user = user.id,
            "TOTP secret from session was not valid base-32"
        );

        session.remove("totp_secret");
        session.set(
            "totp_error",
            "😒 There was a problem with the MFA setup process. Please try again.",
        );

        return Ok(Redirect::see_other("/user/settings/totp").into_response());
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
            user = user.id,
            "TOTP code provided did not match the expected value"
        );

        session.set(
            "totp_error",
            "🤨 The TOTP code you provided was incorrect. Please try again.",
        );

        return Ok(Redirect::see_other("/user/settings/totp").into_response());
    }

    user.set_totp_secret(&env.pool, &secret)
        .await
        .map_err(|err| {
            tracing::error!(user = user.id, err = ?err, "Failed to set TOTP secret");
            InternalServerError(err)
        })?;

    session.remove("totp_secret");
    session.set(
        "password_success",
        "Two-factor authentication has been enabled successfully. Well done 👍",
    );

    Ok(Html("")
        .with_header("HX-Redirect", "/user/settings")
        .into_response())
}

#[handler]
pub fn get_remove_totp(
    env: Data<&Env>,
    user: User,
    csrf_token: &CsrfToken,
) -> poem::Result<Html<String>> {
    render_template(
        "user/remove-totp.html",
        context! {
            csrf_token => csrf_token.0,
            ..authorized_context(&env, &user)
        },
    )
}

#[derive(Debug, Deserialize)]
pub struct RemoveTotpForm {
    csrf_token: String,
}

#[handler]
pub async fn post_remove_totp(
    env: Data<&Env>,
    mut user: User,
    verifier: &CsrfVerifier,
    session: &Session,
    Form(form): Form<RemoveTotpForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&form.csrf_token) {
        tracing::error!("Invalid CSRF token in remove TOTP form");
        return Err(CsrfError.into());
    }

    user.remove_totp_secret(&env.pool).await.map_err(|err| {
        tracing::error!(user = user.id, err = ?err, "Failed to remove TOTP secret");
        InternalServerError(err)
    })?;

    session.set(
        "password_success",
        "Two-factor authentication has been removed from your account! 😲",
    );

    Ok(Html("")
        .with_header("HX-Redirect", "/user/settings")
        .into_response())
}
