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
use validator::{Validate, ValidationError, ValidationErrors};

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
        user::User,
    },
    utils::ValidationErrorsExt,
};

#[handler]
pub async fn get_teams(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
) -> poem::Result<Response> {
    let teams = TeamList::get(&env.pool).await.map_err(|err| {
        tracing::error!(?err, "Failed to get list of teams");
        InternalServerError(err)
    })?;

    Ok(render_template(
        "admin/teams.html",
        context! {
            teams,
            ..authorized_context(&env, &admin)
        },
    )?
    .with_header("HX-Trigger", "closeModals")
    .into_response())
}

#[handler]
pub fn get_new(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
    token: &CsrfToken,
) -> poem::Result<Html<String>> {
    render_template(
        "admin/teams/form.html",
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

    let team_exists = Team::slug_exists(&env.pool, None, &slug)
        .await
        .map_err(InternalServerError)?;

    let user_exists = User::username_exists(&env.pool, None, &slug)
        .await
        .map_err(InternalServerError)?;

    let exists = if team_exists {
        Some("team")
    } else if user_exists {
        Some("user")
    } else {
        None
    };

    render_template(
        "admin/teams/slug.html",
        context! { exists, form => context! { slug } },
    )
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

    let team_exists = Team::slug_exists(&env.pool, Some(team_id), &slug)
        .await
        .map_err(InternalServerError)?;

    let user_exists = User::username_exists(&env.pool, None, &slug)
        .await
        .map_err(InternalServerError)?;

    let exists = if team_exists {
        Some("team")
    } else if user_exists {
        Some("user")
    } else {
        None
    };

    render_template(
        "admin/teams/slug.html",
        context! { exists, team, form => context! { slug } },
    )
}

#[derive(Debug, Deserialize, Validate)]
pub struct NewTeamForm {
    pub token: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 3, max = 100))]
    pub slug: String,
    pub enabled: Option<String>,
    pub limit: Option<i64>,
}

#[handler]
pub async fn post_new(
    env: Data<&Env>,
    next_token: &CsrfToken,
    verifier: &CsrfVerifier,
    SessionAdmin(admin): SessionAdmin,
    Form(form): Form<NewTeamForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&form.token) {
        tracing::error!("Invalid CSRF token in new team form");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let mut errors = ValidationErrors::new();

    if let Err(first_errors) = form.validate() {
        errors.merge(first_errors);
    }

    if let Err(slug_error) = crate::utils::validate_slug(&form.slug) {
        errors.add("slug", slug_error);
    }

    if Team::slug_exists(&env.pool, None, &form.slug)
        .await
        .map_err(InternalServerError)?
    {
        errors.add(
            "slug",
            ValidationError::new("duplicate_slug")
                .with_message("A team with this URL slug already exists".into()),
        );
    }

    if User::username_exists(&env.pool, None, &form.slug)
        .await
        .map_err(InternalServerError)?
    {
        errors.add(
            "slug",
            ValidationError::new("duplicate_username")
                .with_message("A user with this username already exists".into()),
        );
    }

    if !errors.is_empty() {
        return Ok(render_template(
            "admin/teams/form.html",
            context! {
                errors,
                token => next_token.0,
                form => context !{
                    name => &form.name,
                    slug => &form.slug,
                    enabled => form.enabled.as_deref() == Some("on"),
                    limit => form.limit,
                },
                ..authorized_context(&env, &admin)
            },
        )?
        .with_header("HX-Retarget", "#team-form")
        .with_header("HX-Reselect", "#team-form")
        .into_response());
    }

    let NewTeamForm {
        name,
        slug,
        enabled,
        limit,
        ..
    } = form;

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

    Ok(Redirect::see_other("/admin/teams").into_response())
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
        "admin/teams/form.html",
        context! {
            token => token.0,
            team,
            ..authorized_context(&env, &admin)
        },
    )
}

#[derive(Debug, Deserialize, Validate)]
pub struct EditTeamForm {
    pub token: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 3, max = 100))]
    pub slug: String,
    pub enabled: Option<String>,
    pub limit: Option<i64>,
}

#[handler]
pub async fn post_team(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
    next_token: &CsrfToken,
    verifier: &CsrfVerifier,
    Path(team_id): Path<Key<Team>>,
    Form(form): Form<EditTeamForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&form.token) {
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

    let mut errors = ValidationErrors::new();
    if let Err(first_errors) = form.validate() {
        errors.merge(first_errors);
    }

    if Team::slug_exists(&env.pool, Some(team_id), &form.slug)
        .await
        .map_err(InternalServerError)?
    {
        errors.add(
            "slug",
            ValidationError::new("duplicate_slug")
                .with_message("A team with this URL slug already exists".into()),
        );
    }

    if User::username_exists(&env.pool, None, &form.slug)
        .await
        .map_err(InternalServerError)?
    {
        errors.add(
            "slug",
            ValidationError::new("duplicate_username")
                .with_message("A user with this username already exists".into()),
        );
    }

    if !errors.is_empty() {
        return Ok(render_template(
            "admin/teams/form.html",
            context! {
                errors,
                team,
                token => next_token.0,
                form => context !{
                    name => &form.name,
                    slug => &form.slug,
                    enabled => form.enabled.as_deref() == Some("on"),
                    limit => form.limit,
                },
                ..authorized_context(&env, &admin)
            },
        )?
        .with_header("HX-Retarget", "#team-form")
        .with_header("HX-Reselect", "#team-form")
        .into_response());
    }

    let EditTeamForm {
        name,
        slug,
        enabled,
        limit,
        ..
    } = form;

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
