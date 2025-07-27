use poem::{
    error::InternalServerError,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path},
    IntoResponse,
};
use serde::Deserialize;
use validator::{Validate, ValidationError, ValidationErrors};

use parcel_model::{
    team::{Team, TeamMember},
    types::Key,
    user::User,
};

use crate::{
    app::{
        errors::CsrfError,
        extractors::user::SessionUser,
        templates::{authorized_context, render_template},
    },
    env::Env,
    utils::ValidationErrorsExt,
};

#[poem::handler]
pub async fn get_settings(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_token: &CsrfToken,
    Path(id): Path<Key<Team>>,
) -> poem::Result<Html<String>> {
    let Some(team) = Team::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(%id, ?err, "Unable to get team by ID or slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%id, "Team with this URL slug does not exist");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let Some(membership) = TeamMember::get_for_user_and_team(&env.pool, user.id, team.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, %team.id, ?err, "Unable to get team membership");
            InternalServerError(err)
        })?
    else {
        tracing::error!(%user.id, %team.id, "User is not a member of team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    };

    if !membership.can_config {
        tracing::error!(%user.id, %team.id, "User does not have permission to configure team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    let members = TeamMember::get_for_team(&env.pool, team.id)
        .await
        .map_err(|err| {
            tracing::error!(%team.id, ?err, "Unable to get team members");
            InternalServerError(err)
        })?;

    render_template(
        "teams/settings.html",
        minijinja::context! {
            team,
            members,
            csrf_token => csrf_token.0,
            ..authorized_context(&env, &user)
        },
    )
    .await
}

#[derive(Debug, Deserialize, Validate)]
pub struct SettingsForm {
    csrf_token: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 3, max = 100))]
    pub slug: String,
    pub members: String,
}

#[poem::handler]
pub async fn post_settings(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    next_token: &CsrfToken,
    verifier: &CsrfVerifier,
    Path(id): Path<Key<Team>>,
    Form(form): Form<SettingsForm>,
) -> poem::Result<poem::Response> {
    if !verifier.is_valid(&form.csrf_token) {
        tracing::error!("Invalid CSRF token in team settings form");
        return Err(CsrfError.into());
    }

    let Some(mut team) = Team::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(%id, ?err, "Unable to get team by ID or slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%id, "Team with this URL slug does not exist");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let Some(membership) = TeamMember::get_for_user_and_team(&env.pool, user.id, team.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, %team.id, ?err, "Unable to get team membership");
            InternalServerError(err)
        })?
    else {
        tracing::error!(%user.id, %team.id, "User is not a member of team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    };

    if !membership.can_config {
        tracing::error!(%user.id, %team.id, "User does not have permission to configure team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    let mut errors = ValidationErrors::new();

    if let Err(first_errors) = form.validate() {
        errors.merge(first_errors);
    }

    if Team::slug_exists(&env.pool, Some(id), &form.slug)
        .await
        .map_err(InternalServerError)?
    {
        errors.add(
            "slug",
            ValidationError::new("duplicate_slug")
                .with_message("This slug is already in use".into()),
        );
    }
    if User::username_exists(&env.pool, None, &form.slug)
        .await
        .map_err(InternalServerError)?
    {
        errors.add(
            "slug",
            ValidationError::new("duplicate_username")
                .with_message("This slug is already in use".into()),
        );
    }

    if !errors.is_empty() {
        return Ok(render_template(
            "teams/settings.html",
            minijinja::context! {
                errors,
                team,
                token => next_token.0,
                form => minijinja::context! {
                    name => &form.name,
                    slug => &form.slug,
                },
                ..authorized_context(&env, &user)
            },
        )
        .await?
        .with_header("HX-Retarget", "#team-settings-form")
        .with_header("HX-Reselect", "#team-settings-form")
        .into_response());
    }

    let SettingsForm {
        name,
        slug,
        members,
        ..
    } = form;

    team.update(&env.pool, &name, &slug, team.limit, team.enabled)
        .await
        .map_err(|err| {
            tracing::error!(?err, %id, "Failed to update team");
            InternalServerError(err)
        })?;

    tracing::info!(team = %team.id, ?team.name, ?team.slug, "Updated team settings");

    Ok(Html("")
        .with_header("HX-Redirect", format!("/teams/{}", team.slug))
        .into_response())
}

#[derive(Debug, Deserialize)]
pub struct CheckSlugForm {
    csrf_token: String,
    slug: String,
}

#[poem::handler]
pub async fn post_check_slug(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Team>>,
    Form(CheckSlugForm { csrf_token, slug }): Form<CheckSlugForm>,
) -> poem::Result<Html<String>> {
    if !verifier.is_valid(&csrf_token) {
        tracing::error!("CSRF token is invalid in team slug check");
        return Err(CsrfError.into());
    }

    let Some(team) = Team::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(?err, %id, "Failed to get team by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%id, "Team does not exist");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let Some(membership) = TeamMember::get_for_user_and_team(&env.pool, user.id, team.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, %team.id, ?err, "Unable to get team membership");
            InternalServerError(err)
        })?
    else {
        tracing::error!(%user.id, %team.id, "User is not a member of team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    };

    if !membership.can_config {
        tracing::error!(%user.id, %team.id, "User does not have permission to configure team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    let slug = slug.trim().to_string();

    let team_exists = Team::slug_exists(&env.pool, Some(id), &slug)
        .await
        .map_err(InternalServerError)?;

    let user_exists = User::username_exists(&env.pool, None, &slug)
        .await
        .map_err(InternalServerError)?;

    let exists = team_exists || user_exists;

    render_template(
        "teams/slug.html",
        minijinja::context! { exists, team, form => minijinja::context! { slug } },
    )
    .await
}
