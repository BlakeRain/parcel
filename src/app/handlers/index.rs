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
    model::team::Team,
};

use super::uploads::ListQuery;

#[handler]
pub async fn get_index(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Query(query): Query<ListQuery>,
) -> poem::Result<impl IntoResponse> {
    let teams = Team::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get teams for user");
            poem::error::InternalServerError(err)
        })?;

    render_template(
        "index.html",
        minijinja::context! {
            query,
            teams,
            index_js => javascript!("scripts/index.ts", format = "esm"),
            ..authorized_context(&env, &user)
        },
    )
}
