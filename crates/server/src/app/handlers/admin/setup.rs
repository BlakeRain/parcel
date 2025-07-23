use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::{header::LOCATION, HeaderMap, HeaderValue, StatusCode},
    session::Session,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Redirect},
    IntoResponse, Response,
};
use serde::Deserialize;
use time::OffsetDateTime;
use validator::Validate;

use parcel_model::{
    password::StoredPassword,
    types::Key,
    upload::UploadOrder,
    user::{requires_setup, User},
};

use crate::{
    app::templates::{default_context, render_template},
    env::Env,
    utils::validate_slug,
};

#[handler]
pub async fn get_setup(
    env: Data<&Env>,
    token: &CsrfToken,
) -> poem::Result<(StatusCode, HeaderMap, Html<String>)> {
    let required = requires_setup(&env.pool)
        .await
        .map_err(InternalServerError)?;
    if !required {
        tracing::warn!("Setup requested, but was already completed");

        return Ok((
            StatusCode::SEE_OTHER,
            HeaderMap::from_iter([(LOCATION, HeaderValue::from_static("/admin"))]),
            Html("Goto <a href=\"/admin\">administration</a>".to_string()),
        ));
    }

    let body = render_template(
        "admin/setup.html",
        context! {
            token => token.0,
            ..default_context(&env)
        },
    )
    .await?;

    Ok((StatusCode::OK, HeaderMap::new(), body))
}

#[derive(Debug, Deserialize, Validate)]
pub struct SetupForm {
    token: String,
    #[validate(length(min = 3), custom(function = "validate_slug"))]
    username: String,
    #[validate(length(min = 8))]
    password: String,
}

#[handler]
pub async fn post_setup(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    session: &Session,
    Form(form): Form<SetupForm>,
) -> poem::Result<Response> {
    let required = requires_setup(&env.pool)
        .await
        .map_err(InternalServerError)?;

    if !required {
        tracing::error!("Setup form submitted, but setup was already completed");

        return Ok(render_template(
            "admin/setup.html",
            context! {
                error => "Setup is no longer required",
                ..default_context(&env)
            },
        )
        .await?
        .into_response());
    }

    if !verifier.is_valid(&form.token) {
        tracing::error!("CSRF token in setup form was invalid");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    if let Err(err) = form.validate() {
        tracing::error!(error = ?err, "Validation error in setup form");

        return Ok(render_template(
            "admin/setup.html",
            context! {
                error => "There was an error in your submission",
                errors => err,
                ..default_context(&env)
            },
        )
        .await?
        .into_response());
    }

    let SetupForm {
        username, password, ..
    } = form;

    let name = username.clone();
    let now = OffsetDateTime::now_utc();
    let admin = User {
        id: Key::new(),
        username,
        name,
        password: StoredPassword::new(&password)?,
        totp: None,
        enabled: true,
        admin: true,
        limit: None,
        created_at: now,
        created_by: None,
        last_access: Some(now),
        default_order: UploadOrder::UploadedAt,
        default_asc: false,
    };

    admin.create(&env.pool).await.map_err(|err| {
        tracing::error!(
            admin = %admin.id,
            username = ?admin.username,
            err = ?err,
            "Failed to create new administrator"
        );

        InternalServerError(err)
    })?;

    tracing::info!(admin = %admin.id, "Created initial administrator");
    session.set("user_id", admin.id);

    Ok(Redirect::see_other("/admin").into_response())
}
