use poem::{
    endpoint::StaticFilesEndpoint,
    middleware::{Csrf, Tracing},
    session::{CookieConfig, CookieSession},
    web::cookie::{CookieKey, SameSite},
    EndpointExt, IntoEndpoint,
};
use poem_route_macro::define_routes;

use crate::env::Env;

mod extractors {
    pub mod admin;
}

pub mod errors;
pub mod templates;

mod handlers {
    pub mod admin;
    pub mod index;
    pub mod uploads;
    pub mod users;
}

pub fn create_app(env: Env, cookie_key: Option<&[u8]>) -> impl IntoEndpoint {
    let cookie_key = if let Some(key) = cookie_key {
        CookieKey::derive_from(key)
    } else {
        tracing::info!("Generating new cookie key (no 'COOKIE_SECRET' or '--cookie-secret')");
        CookieKey::generate()
    };

    define_routes!({
        *"/static" { StaticFilesEndpoint::new("./static") }

        "/"                             handlers::index::index                GET
        "/uploads/list"                 handlers::uploads::list               GET      DELETE
        "/uploads/new"                  handlers::uploads::new                GET POST
        "/uploads/stats"                handlers::uploads::stats              GET
        "/uploads/:id"                  handlers::uploads::upload             GET      DELETE
        "/uploads/:id/edit"             handlers::uploads::edit               GET POST
        "/uploads/:id/public"           handlers::uploads::public                 POST
        "/uploads/:id/reset"            handlers::uploads::reset                  POST
        "/uploads/:id/download"         handlers::uploads::download           GET
        "/user/signin"                  handlers::users::signin               GET POST
        "/user/signin/totp"             handlers::users::signin_totp          GET POST
        "/user/signout"                 handlers::users::signout              GET
        "/user/settings"                handlers::users::settings             GET POST
        "/user/settings/password"       handlers::users::password                 POST
        "/user/settings/totp"           handlers::users::setup_totp           GET POST
        "/user/settings/totp/remove"    handlers::users::remove_totp          GET POST
        "/admin"                        handlers::admin::admin                GET
        "/admin/setup"                  handlers::admin::setup::setup         GET POST
        "/admin/uploads"                handlers::admin::uploads::uploads     GET
        "/admin/users"                  handlers::admin::users::users         GET
        "/admin/users/new"              handlers::admin::users::new           GET POST
        "/admin/users/:id"              handlers::admin::users::user          GET POST DELETE
        "/admin/users/:id/disable"      handlers::admin::users::disable_user      POST
        "/admin/users/:id/enable"       handlers::admin::users::enable_user       POST
    })
    .catch_error(errors::NotSignedInError::handle)
    .catch_error(errors::CsrfError::handle)
    .data(env)
    .with(Csrf::new())
    .with(Tracing)
    .with(CookieSession::new(
        CookieConfig::private(cookie_key)
            .name("parcel")
            .same_site(Some(SameSite::Strict))
            .max_age(Some(std::time::Duration::from_secs(14 * 24 * 60 * 60))),
    ))
}
