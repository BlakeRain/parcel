use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path, Redirect},
    IntoResponse, Response,
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    app::{
        extractors::admin::SessionAdmin,
        templates::{authorized_context, render_404, render_template},
    },
    env::Env,
    model::{
        team::{Team, TeamList},
        types::Key,
        upload::Upload,
    },
};

#[handler]
pub async fn get_teams(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
) -> poem::Result<Html<String>> {
    let teams = TeamList::get(&env.pool).await.map_err(|err| {
        tracing::error!(?err, "Failed to get list of teams");
        InternalServerError(err)
    })?;

    render_template(
        "admin/teams.html",
        context! {
            teams,
            ..authorized_context(&env, &admin)
        },
    )
}

#[handler]
pub fn get_new(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
    token: &CsrfToken,
) -> poem::Result<Html<String>> {
    render_template(
        "admin/teams/new.html",
        context! {
            token => token.0,
            ..authorized_context(&env, &admin)
        },
    )
}

#[derive(Debug, Deserialize)]
pub struct CheckSlugForm {
    token: String,
    slug: String,
}

#[handler]
pub async fn post_check_new_slug(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    SessionAdmin(_): SessionAdmin,
    Form(CheckSlugForm { token, slug }): Form<CheckSlugForm>,
) -> poem::Result<Html<String>> {
    if !verifier.is_valid(&token) {
        tracing::error!("CSRF token is invalid in team slug check");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let slug = slug.trim().to_string();
    let exists = Team::slug_exists(&env.pool, None, &slug)
        .await
        .map_err(InternalServerError)?;

    render_template("admin/teams/slug.html", context! { exists, slug })
}

#[handler]
pub async fn post_check_slug(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    SessionAdmin(_): SessionAdmin,
    Path(team_id): Path<Key<Team>>,
    Form(CheckSlugForm { token, slug }): Form<CheckSlugForm>,
) -> poem::Result<Html<String>> {
    if !verifier.is_valid(&token) {
        tracing::error!("CSRF token is invalid in team slug check");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(?err, %team_id, "Failed to get team by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%team_id, "Team does not exist");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let slug = slug.trim().to_string();
    let exists = Team::slug_exists(&env.pool, Some(team_id), &slug)
        .await
        .map_err(InternalServerError)?;

    render_template("admin/teams/slug.html", context! { exists, team, slug })
}

#[derive(Debug, Deserialize)]
pub struct NewTeamForm {
    pub token: String,
    pub name: String,
    pub slug: String,
    pub enabled: Option<String>,
    pub limit: Option<i64>,
}

#[handler]
pub async fn post_new(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    SessionAdmin(admin): SessionAdmin,
    Form(NewTeamForm {
        token,
        name,
        slug,
        enabled,
        limit,
    }): Form<NewTeamForm>,
) -> poem::Result<impl IntoResponse> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in new team form");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let enabled = enabled.as_deref() == Some("on");
    let team = Team {
        id: Key::new(),
        name,
        slug,
        enabled,
        limit: limit.map(|limit| limit * 1024 * 1024),
        created_at: OffsetDateTime::now_utc(),
        created_by: admin.id,
    };

    team.create(&env.pool).await.map_err(|err| {
        tracing::error!(%admin.id, ?err, "Failed to create new team");
        InternalServerError(err)
    })?;

    tracing::info!(team = %team.id, ?team.name, "Created new team");

    Ok(Redirect::see_other("/admin/teams"))
}

#[handler]
pub async fn get_team(
    env: Data<&Env>,
    token: &CsrfToken,
    Path(team_id): Path<Key<Team>>,
    SessionAdmin(admin): SessionAdmin,
) -> poem::Result<Html<String>> {
    let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(?err, %team_id, "Failed to get team");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%team_id, "Unrecognized team ID");
        return render_404("Unrecognized team ID");
    };

    render_template(
        "admin/teams/new.html",
        context! {
            token => token.0,
            team,
            ..authorized_context(&env, &admin)
        },
    )
}

#[derive(Debug, Deserialize)]
pub struct EditTeamForm {
    pub token: String,
    pub name: String,
    pub slug: String,
    pub enabled: Option<String>,
    pub limit: Option<i64>,
}

#[handler]
pub async fn post_team(
    env: Data<&Env>,
    SessionAdmin(_): SessionAdmin,
    verifier: &CsrfVerifier,
    Path(team_id): Path<Key<Team>>,
    Form(EditTeamForm {
        token,
        name,
        slug,
        enabled,
        limit,
    }): Form<EditTeamForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in edit team form");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(mut team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(?err, %team_id, "Failed to get team");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%team_id, "Unrecognized team ID");
        return Ok(render_404("Unrecognized team ID")?.into_response());
    };

    let enabled = enabled.as_deref() == Some("on");
    let limit = limit.map(|limit| limit * 1024 * 1024);
    team.update(&env.pool, &name, &slug, limit, enabled)
        .await
        .map_err(|err| {
            tracing::error!(?err, %team_id, "Failed to update team");
            InternalServerError(err)
        })?;

    tracing::info!(team = %team.id, ?team.name, "Updated team");

    Ok(Redirect::see_other("/admin/teams").into_response())
}

#[handler]
pub async fn delete_team(
    env: Data<&Env>,
    SessionAdmin(_): SessionAdmin,
    Path(team_id): Path<Key<Team>>,
) -> poem::Result<Redirect> {
    let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(%team_id, ?err, "Failed to get team by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%team_id, "Team does not exist");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let upload_slugs = Upload::delete_for_team(&env.pool, team_id)
        .await
        .map_err(|err| {
            tracing::error!(?err, %team_id, "Failed to delete team uploads");
            InternalServerError(err)
        })?;

    for slug in upload_slugs {
        let path = env.cache_dir.join(&slug);
        tracing::info!(?path, owner = %team_id, "Deleting cached upload");
        if let Err(err) = tokio::fs::remove_file(&path).await {
            tracing::error!(?path, ?err, %team_id, "Failed to delete cached upload");
        }
    }

    team.delete(&env.pool).await.map_err(|err| {
        tracing::error!(%team_id, ?err, "Failed to delete team");
        InternalServerError(err)
    })?;

    Ok(Redirect::see_other("/admin/teams"))
}
