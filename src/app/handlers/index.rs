use esbuild_bundle::javascript;
use poem::{
    handler,
    web::{Data, Query},
    IntoResponse,
};

use crate::{
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, render_template},
    },
    env::Env,
};

use super::uploads::ListQuery;

#[handler]
pub fn get_index(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Query(query): Query<ListQuery>,
) -> poem::Result<impl IntoResponse> {
    render_template(
        "index.html",
        minijinja::context! {
            query,
            index_js => javascript!("scripts/index.ts"),
            ..authorized_context(&env, &user)
        },
    )
}
