use poem::{
    endpoint::StaticFilesEndpoint,
    get,
    middleware::Csrf,
    post, put,
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
    pub mod uploads;
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
        //
        //
        // --- Uploads ----------------------------------------------------------------------------
        .at("/", get(handlers::index::get_index))
        //
        // POST /uploads
        .at("/uploads", post(handlers::uploads::post_new_upload))
        //
        // GET  /uploads/:id
        // POST /uploads/:id
        .at(
            "/uploads/:id",
            get(handlers::uploads::get_upload).post(handlers::uploads::post_upload),
        )
        //
        //
        // --- Users ------------------------------------------------------------------------------
        //
        // GET  /user/signin
        // POST /user/signin
        .at(
            "/user/signin",
            get(handlers::users::get_signin).post(handlers::users::post_signin),
        )
        //
        // GET  /user/settings
        // POST /user/settings
        // POST /user/settings/password
        .at(
            "/user/settings",
            get(handlers::users::get_settings).post(handlers::users::post_settings),
        )
        .at(
            "/user/settings/password",
            post(handlers::users::post_settings_password),
        )
        //
        // GET /user/signout
        .at("/user/signout", get(handlers::users::get_signout))
        //
        //
        // --- Administration ---------------------------------------------------------------------
        //
        // GET /admin
        .at("/admin", get(handlers::admin::get_admin))
        //
        // GET /admin/setup
        // PUT /admin/setup
        .at(
            "/admin/setup",
            get(handlers::admin::setup::get_setup).post(handlers::admin::setup::post_setup),
        )
        //
        // GET /admin/users
        // GET /admin/users/new
        .at("/admin/users", get(handlers::admin::users::get_users))
        .at(
            "/admin/users/new",
            get(handlers::admin::users::get_users_new).post(handlers::admin::users::post_users_new),
        )
        //
        // GET    /admin/users/:id
        // PUT    /admin/users/:id
        // DELETE /admin/users/:id
        .at(
            "/admin/users/:id",
            get(handlers::admin::users::get_user_edit)
                .put(handlers::admin::users::put_user)
                .delete(handlers::admin::users::delete_user),
        )
        //
        // PUT /admin/users/:id/disable
        .at(
            "/admin/users/:id/disable",
            put(handlers::admin::users::put_disable_user),
        )
        //
        // PUT /admin/users/:id/enable
        .at(
            "/admin/users/:id/enable",
            put(handlers::admin::users::put_enable_user),
        )
        //
        // GET /admin/uploads
        .at("/admin/uploads", get(handlers::admin::uploads::get_uploads))
        .catch_error(errors::NotSignedInError::handle)
        .catch_error(errors::CsrfError::handle)
        .data(env)
        .with(Csrf::new())
        .with(CookieSession::new(CookieConfig::private(cookie_key)))
}
