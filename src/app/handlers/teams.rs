use esbuild_bundle::javascript;
use poem::{
    error::InternalServerError,
    handler,
    web::{Data, Html, Path, Query},
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

pub mod uploads;

#[handler]
pub async fn get_team(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(slug): Path<String>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let teams = Team::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get teams for user");
            InternalServerError(err)
        })?;

    let team = match teams.iter().find(|team| team.slug == slug) {
        Some(team) => team,
        None => {
            tracing::error!(?slug, "Team with this URL slug does not exist");
            return Err(poem::Error::from_status(poem::http::StatusCode::NOT_FOUND));
        }
    };

    render_template(
        "index.html",
        minijinja::context! {
            query,
            team,
            teams,
            index_js => javascript!("scripts/index.ts", format = "esm"),
            ..authorized_context(&env, &user)
        },
    )
}
