use poem::{
    endpoint::StaticFilesEndpoint,
    get,
    middleware::Csrf,
    session::{CookieConfig, CookieSession},
    web::cookie::CookieKey,
    EndpointExt, IntoEndpoint, Route,
};

use crate::env::Env;

mod extractors {
    pub mod admin;
}

pub mod errors;
pub mod templates;

mod handlers {
    pub mod admin;
    pub mod index;
    pub mod users;
}

pub fn create_app(env: Env, cookie_key: Option<&[u8]>) -> impl IntoEndpoint {
    let cookie_key = if let Some(key) = cookie_key {
        CookieKey::derive_from(key)
    } else {
        tracing::info!("Generating new cookie key");
        CookieKey::generate()
    };

    Route::new()
        .nest("/static", StaticFilesEndpoint::new("./static"))
        .at("/", get(handlers::index::get_index))
        // GET  /user/signin
        // POST /user/signin
        .at(
            "/user/signin",
            get(handlers::users::get_signin).post(handlers::users::post_signin),
        )
        // GET /user/settings
        .at("/user/settings", get(handlers::users::get_settings))
        // GET /user/signout
        .at("/user/signout", get(handlers::users::get_signout))
        // GET /admin
        .at("/admin", get(handlers::admin::get_admin))
        // GET /admin/setup
        // PUT /admin/setup
        .at(
            "/admin/setup",
            get(handlers::admin::setup::get_setup).post(handlers::admin::setup::post_setup),
        )
        .catch_error(errors::NotSignedInError::handle)
        .catch_error(errors::CsrfError::handle)
        .data(env)
        .with(Csrf::new())
        .with(CookieSession::new(CookieConfig::private(cookie_key)))
}

// GET    /admin/users
// GET    /admin/users/new
// POST   /admin/users/new
// GET    /admin/users/:id/edit
// PUT    /admin/users/:id
// GET    /admin/users/:id/delete
// DELETE /admin/users/:id
//
// GET    /admin/uploads
// GET    /admin/uploads/:id/edit
// POST   /admin/uploads/:id
// GET    /admin/uploads/:id/delete
// DELETE /admin/uploads/:id
// GET    /
// GET    /uploads
// POST   /uploads
// GET    /uploads/:slug
// DELETE /uploads/:slug
