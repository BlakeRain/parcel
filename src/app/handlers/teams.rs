use esbuild_bundle::javascript;
use poem::{
    handler,
    web::{Data, Html, Path, Query},
};

use crate::{
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{team::Team, types::Key},
};

use super::uploads::ListQuery;

pub mod uploads;

#[handler]
pub async fn get_team(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Team>>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let teams = Team::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get teams for user");
            poem::error::InternalServerError(err)
        })?;

    let Some(team) = teams.iter().find(|team| team.id == id) else {
        return Err(poem::Error::from_status(poem::http::StatusCode::NOT_FOUND));
    };

    render_template(
        "index.html",
        minijinja::context! {
            query,
            team,
            teams,
            index_js => javascript!("scripts/index.ts"),
            ..authorized_context(&env, &user)
        },
    )
}
