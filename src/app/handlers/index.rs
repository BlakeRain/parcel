use esbuild_bundle::javascript;
use poem::{
    handler,
    web::{Data, Query},
    IntoResponse,
};

use crate::{
    app::templates::{authorized_context, render_template},
    env::Env,
    model::user::User,
};

use super::uploads::ListQuery;

#[handler]
pub fn get_index(
    env: Data<&Env>,
    user: User,
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
