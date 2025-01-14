use poem::{
    endpoint::StaticFilesEndpoint,
    middleware::{Cors, Csrf, NormalizePath, Tracing, TrailingSlash},
    session::{CookieConfig, CookieSession},
    web::cookie::{CookieKey, SameSite},
    EndpointExt, IntoEndpoint,
};
use poem_route_macro::define_routes;

use crate::env::Env;

mod extractors {
    pub mod admin;
    pub mod form;
    pub mod user;
}

pub mod errors;
pub mod templates;

mod handlers {
    pub mod admin;
    pub mod index;
    pub mod teams;
    pub mod uploads;
    pub mod users;
    pub mod utils;

    #[cfg(debug_assertions)]
    pub mod debug;
}

fn add_tailwind_rebuilder<E: IntoEndpoint>(endpoint: E) -> anyhow::Result<impl IntoEndpoint> {
    #[cfg(debug_assertions)]
    {
        use anyhow::Context;
        use templates::tailwind::TailwindRebuilder;

        let rebuilder = TailwindRebuilder::new(".", ["templates", "style"])
            .context("failed to create tailwind rebuilder")?;

        Ok(endpoint.data(rebuilder))
    }

    #[cfg(not(debug_assertions))]
    {
        Ok(endpoint)
    }
}

#[cfg(debug_assertions)]
use handlers::debug::add_debug_routes;

#[cfg(not(debug_assertions))]
fn add_debug_routes(app: poem::Route) -> poem::Route {
    app
}

pub fn create_app(env: Env, cookie_key: Option<&[u8]>) -> anyhow::Result<impl IntoEndpoint> {
    let cookie_key = if let Some(key) = cookie_key {
        CookieKey::derive_from(key)
    } else {
        tracing::info!("Generating new cookie key (no 'COOKIE_SECRET' or '--cookie-secret')");
        CookieKey::generate()
    };

    let routes = add_debug_routes(define_routes!({
        *"/static" { StaticFilesEndpoint::new("./static") }

        "/"                             handlers::index::index                  GET
        "/tab"                          handlers::index::tab                    GET
        "/uploads/delete"               handlers::uploads::delete                   POST
        "/uploads/list"                 handlers::uploads::list                 GET
        "/uploads/list/:page"           handlers::uploads::page                 GET
        "/uploads/new"                  handlers::uploads::new                  GET POST
        "/uploads/:id"                  handlers::uploads::upload               GET      DELETE
        "/uploads/:id/edit"             handlers::uploads::edit                 GET POST
        "/uploads/:id/edit/slug"        handlers::uploads::check_slug               POST
        "/uploads/:id/public"           handlers::uploads::public                   POST
        "/uploads/:id/reset"            handlers::uploads::reset                    POST
        "/uploads/:id/share"            handlers::uploads::share                GET
        "/uploads/:id/transfer"         handlers::uploads::transfer             GET POST
        "/uploads/:id/download"         handlers::uploads::download             GET POST
        "/uploads/:owner/:slug"         handlers::uploads::custom_upload        GET
        "/teams/:id"                    handlers::teams::team                   GET
        "/teams/:id/tab"                handlers::teams::tab                    GET
        "/teams/:id/uploads/list"       handlers::teams::uploads::list          GET
        "/teams/:id/uploads/list/:page" handlers::teams::uploads::page          GET
        "/user/signin"                  handlers::users::signin                 GET POST
        "/user/signin/totp"             handlers::users::signin_totp            GET POST
        "/user/signout"                 handlers::users::signout                GET
        "/user/settings"                handlers::users::settings               GET POST
        "/user/settings/password"       handlers::users::password                   POST
        "/user/settings/totp"           handlers::users::setup_totp             GET POST
        "/user/settings/totp/remove"    handlers::users::remove_totp            GET POST
        "/admin"                        handlers::admin::admin                  GET
        "/admin/setup"                  handlers::admin::setup::setup           GET POST
        "/admin/uploads"                handlers::admin::uploads::uploads       GET
        "/admin/uploads/cache"          handlers::admin::uploads::cache         GET POST DELETE
        "/admin/users"                  handlers::admin::users::users           GET
        "/admin/users/new"              handlers::admin::users::new             GET POST
        "/admin/users/new/username"     handlers::admin::users::new_username        POST
        "/admin/users/:id"              handlers::admin::users::user            GET POST DELETE
        "/admin/users/:id/disable"      handlers::admin::users::disable_user        POST
        "/admin/users/:id/enable"       handlers::admin::users::enable_user         POST
        "/admin/users/:id/masquerade"   handlers::admin::users::masquerade      GET
        "/admin/users/:id/username"     handlers::admin::users::check_username      POST
        "/admin/teams"                  handlers::admin::teams::teams           GET
        "/admin/teams/new"              handlers::admin::teams::new             GET POST
        "/admin/teams/new/slug"         handlers::admin::teams::check_new_slug      POST
        "/admin/teams/:id"              handlers::admin::teams::team            GET POST
        "/admin/teams/:id/slug"         handlers::admin::teams::check_slug          POST
    }));

    let routes = add_tailwind_rebuilder(routes)?.into_endpoint();

    Ok(routes
        .with(NormalizePath::new(TrailingSlash::Trim))
        .catch_error(errors::NotSignedInError::handle)
        .catch_error(errors::CsrfError::handle)
        .catch_error(errors::handle_404)
        .catch_all_error(errors::handle_500)
        .data(env)
        .with(Cors::new())
        .with(Csrf::new().cookie_name("parcel-csrf"))
        .with(Tracing)
        .with(CookieSession::new(
            CookieConfig::private(cookie_key)
                .name("parcel")
                .same_site(Some(SameSite::Strict))
                .max_age(Some(std::time::Duration::from_secs(14 * 24 * 60 * 60))),
        )))
}
