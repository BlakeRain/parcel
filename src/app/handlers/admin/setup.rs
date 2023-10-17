use poem::{
    error::InternalServerError,
    handler,
    http::{header::LOCATION, HeaderMap, HeaderValue, StatusCode},
    session::Session,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html},
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    app::templates::{default_context, render_template},
    env::Env,
    model::user::{hash_password, requires_setup, User},
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
        return Ok((
            StatusCode::SEE_OTHER,
            HeaderMap::from_iter([(LOCATION, HeaderValue::from_static("/admin"))]),
            Html("Goto <a href=\"/admin\">administration</a>".to_string()),
        ));
    }

    let mut context = default_context();
    context.insert("token", &token.0);
    let body = render_template("admin/setup.html", &context)?;
    Ok((StatusCode::OK, HeaderMap::new(), body))
}

#[derive(Debug, Deserialize)]
pub struct SetupForm {
    token: String,
    username: String,
    password: String,
}

#[handler]
pub async fn post_setup(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    session: &Session,
    Form(SetupForm {
        token,
        username,
        password,
    }): Form<SetupForm>,
) -> poem::Result<(StatusCode, HeaderMap, Html<String>)> {
    let required = requires_setup(&env.pool)
        .await
        .map_err(InternalServerError)?;

    if !required {
        tracing::info!("Setup already completed");
        let mut context = default_context();
        context.insert("error", &true);
        let body = render_template("admin/setup.html", &context)?;
        return Ok((StatusCode::OK, HeaderMap::new(), body));
    }

    if !verifier.is_valid(&token) {
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let mut admin = User {
        id: 0,
        username,
        password: hash_password(&password),
        enabled: true,
        admin: true,
        created_at: OffsetDateTime::now_utc(),
        created_by: None,
    };

    admin.create(&env.pool).await.map_err(InternalServerError)?;
    session.set("user_id", admin.id);

    Ok((
        StatusCode::SEE_OTHER,
        HeaderMap::from_iter([(LOCATION, HeaderValue::from_static("/admin"))]),
        Html("Goto <a href=\"/admin\">administration</a>".to_string()),
    ))
}
