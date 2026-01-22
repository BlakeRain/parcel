use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path, Query, Redirect},
    IntoResponse, Response,
};
use serde::Deserialize;
use time::OffsetDateTime;
use validator::{Validate, ValidationError, ValidationErrors};

use parcel_model::{
    team::{Team, TeamList},
    types::Key,
    upload::Upload,
    user::User,
};

use crate::{
    app::{
        errors::CsrfError,
        extractors::admin::SessionAdmin,
        handlers::utils::delete_upload_cache_by_slug,
        templates::{authorized_context, render_template},
    },
    env::Env,
    utils::{SizeUnit, ValidationErrorsExt},
};

#[handler]
pub async fn get_teams(
    env: Data<&Env>,
    csrf_token: &CsrfToken,
    SessionAdmin(admin): SessionAdmin,
) -> poem::Result<Response> {
    let teams = TeamList::get_with_pagination(&env.pool, 0, 50).await.map_err(|err| {
        tracing::error!(?err, "Failed to get list of teams");
        InternalServerError(err)
    })?;

    Ok(render_template(
        "admin/teams.html",
        context! {
            teams,
            csrf_token => csrf_token.0,
            page => 0,
            ..authorized_context(&env, &admin)
        },
    )
    .await?
    .with_header("HX-Trigger", "closeModals")
    .into_response())
}

#[handler]
pub async fn get_teams_page(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
    Path(page): Path<u32>,
) -> poem::Result<Html<String>> {
    let teams = TeamList::get_with_pagination(&env.pool, page * 50, 50).await.map_err(|err| {
        tracing::error!(?err, page = page, "Failed to get page of teams");
        InternalServerError(err)
    })?;

    render_template(
        "admin/teams/page.html",
        context! {
            teams,
            page,
            ..authorized_context(&env, &admin)
        },
    )
    .await
}

#[handler]
pub async fn get_new(
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
    .await
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
        return Err(CsrfError.into());
    }

    let slug = slug.trim().to_string();

    let team_exists = Team::slug_exists(&env.pool, None, &slug)
        .await
        .map_err(|err| {
            tracing::error!(?err, %slug, "Failed to check if team slug exists");
            InternalServerError(err)
        })?;

    let user_exists = User::username_exists(&env.pool, None, &slug)
        .await
        .map_err(|err| {
            tracing::error!(?err, %slug, "Failed to check if username exists");
            InternalServerError(err)
        })?;

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
    .await
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
        return Err(CsrfError.into());
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
        .map_err(|err| {
            tracing::error!(?err, %team_id, %slug, "Failed to check if team slug exists");
            InternalServerError(err)
        })?;

    let user_exists = User::username_exists(&env.pool, None, &slug)
        .await
        .map_err(|err| {
            tracing::error!(?err, %slug, "Failed to check if username exists");
            InternalServerError(err)
        })?;

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
    .await
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
    pub limit_unit: Option<SizeUnit>,
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
        return Err(CsrfError.into());
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
        .map_err(|err| {
            tracing::error!(?err, slug = %form.slug, "Failed to check if team slug exists");
            InternalServerError(err)
        })?
    {
        errors.add(
            "slug",
            ValidationError::new("duplicate_slug")
                .with_message("A team with this URL slug already exists".into()),
        );
    }

    if User::username_exists(&env.pool, None, &form.slug)
        .await
        .map_err(|err| {
            tracing::error!(?err, slug = %form.slug, "Failed to check if username exists");
            InternalServerError(err)
        })?
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
                    limit_unit => form.limit_unit,
                },
                ..authorized_context(&env, &admin)
            },
        )
        .await?
        .with_header("HX-Retarget", "#team-form")
        .with_header("HX-Reselect", "#team-form")
        .into_response());
    }

    let NewTeamForm {
        name,
        slug,
        enabled,
        limit,
        limit_unit,
        ..
    } = form;

    let enabled = enabled.as_deref() == Some("on");
    let limit = limit.and_then(|limit| limit_unit.map(|unit| limit * unit.to_bytes()));
    let team = Team {
        id: Key::new(),
        name,
        slug,
        enabled,
        limit,
        created_at: OffsetDateTime::now_utc(),
        created_by: Some(admin.id),
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
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    render_template(
        "admin/teams/form.html",
        context! {
            token => token.0,
            team,
            ..authorized_context(&env, &admin)
        },
    )
    .await
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
    pub limit_unit: Option<SizeUnit>,
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
        return Err(CsrfError.into());
    }

    let Some(mut team) = Team::get(&env.pool, team_id).await.map_err(|err| {
        tracing::error!(?err, %team_id, "Failed to get team");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%team_id, "Unrecognized team ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let mut errors = ValidationErrors::new();
    if let Err(first_errors) = form.validate() {
        errors.merge(first_errors);
    }

    if Team::slug_exists(&env.pool, Some(team_id), &form.slug)
        .await
        .map_err(|err| {
            tracing::error!(?err, %team_id, slug = %form.slug, "Failed to check if team slug exists");
            InternalServerError(err)
        })?
    {
        errors.add(
            "slug",
            ValidationError::new("duplicate_slug")
                .with_message("A team with this URL slug already exists".into()),
        );
    }

    if User::username_exists(&env.pool, None, &form.slug)
        .await
        .map_err(|err| {
            tracing::error!(?err, slug = %form.slug, "Failed to check if username exists");
            InternalServerError(err)
        })?
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
                    limit_unit => form.limit_unit,
                },
                ..authorized_context(&env, &admin)
            },
        )
        .await?
        .with_header("HX-Retarget", "#team-form")
        .with_header("HX-Reselect", "#team-form")
        .into_response());
    }

    let EditTeamForm {
        name,
        slug,
        enabled,
        limit,
        limit_unit,
        ..
    } = form;

    let enabled = enabled.as_deref() == Some("on");
    let limit = limit.and_then(|limit| limit_unit.map(|unit| limit * unit.to_bytes()));
    team.update(&env.pool, &name, &slug, limit, enabled)
        .await
        .map_err(|err| {
            tracing::error!(?err, %team_id, "Failed to update team");
            InternalServerError(err)
        })?;

    tracing::info!(team = %team.id, ?team.name, ?team.slug, "Updated team");

    Ok(Redirect::see_other("/admin/teams").into_response())
}

#[derive(Debug, Deserialize)]
pub struct DeleteTeamQuery {
    csrf_token: String,
}

#[handler]
pub async fn delete_team(
    env: Data<&Env>,
    SessionAdmin(_): SessionAdmin,
    csrf_verifier: &CsrfVerifier,
    Path(team_id): Path<Key<Team>>,
    Query(DeleteTeamQuery { csrf_token }): Query<DeleteTeamQuery>,
) -> poem::Result<Redirect> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::error!("Invalid CSRF token in delete team request");
        return Err(CsrfError.into());
    }

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
        delete_upload_cache_by_slug(&env, &slug).await;
    }

    team.delete(&env.pool).await.map_err(|err| {
        tracing::error!(%team_id, ?err, "Failed to delete team");
        InternalServerError(err)
    })?;

    Ok(Redirect::see_other("/admin/teams"))
}
