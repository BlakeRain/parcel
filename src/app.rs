use poem::{get, EndpointExt, IntoEndpoint, Route};

use crate::env::Env;

mod templates;
mod handlers {
    pub mod index;
    pub mod admin {
        pub mod setup;
        pub mod users;
    }
}

pub fn create_app(env: Env) -> impl IntoEndpoint {
    Route::new()
        .at("/", get(handlers::index::get_index))
        .data(env)
}

// GET    /admin/setup
// POST   /admin/setup
// GET    /admin/users
// POST   /admin/users
// DELETE /admin/users/:id
// GET    /admin/users/:id/edit
// POST   /admin/users/:id/edit
// GET    /admin/uploads
// POST   /admin/uploads/:id
// DELETE /admin/uploads/:id
// GET    /
// GET    /uploads
// POST   /uploads
// GET    /uploads/:slug
// DELETE /uploads/:slug
// GET    /user/signin
// POST   /user/signin
