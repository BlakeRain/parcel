use std::collections::HashSet;

use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Html, Path, Redirect},
    IntoResponse, Response,
};
use serde::Deserialize;
use time::OffsetDateTime;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::{
    app::{
        extractors::{admin::SessionAdmin, form::Form},
        templates::{authorized_context, render_404, render_template},
    },
    env::Env,
    model::{
        team::{Team, TeamSelect},
        types::Key,
        upload::Upload,
        user::{hash_password, User},
    },
    utils::ValidationErrorsExt,
};

#[handler]
pub async fn get_users(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
) -> poem::Result<Response> {
    let users = User::get_list(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, "Failed to get list of users");
        InternalServerError(err)
    })?;

    Ok(render_template(
        "admin/users.html",
        context! {
            users,
            ..authorized_context(&env, &admin)
        },
    )?
    .with_header("HX-Trigger", "closeModals")
    .into_response())
}

#[handler]
pub async fn get_new(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
    token: &CsrfToken,
) -> poem::Result<Html<String>> {
    let teams = TeamSelect::get(&env.pool).await.map_err(|err| {
        tracing::error!(?err, "Failed to get team selection");
        InternalServerError(err)
    })?;

    render_template(
        "admin/users/form.html",
        context! {
            token => token.0,
            teams,
            ..authorized_context(&env, &admin)
        },
    )
}

#[derive(Debug, Deserialize, Validate)]
pub struct NewUserForm {
    token: String,
    #[validate(length(min = 3, max = 100))]
    username: String,
    #[validate(length(min = 3, max = 100))]
    name: String,
    #[validate(length(min = 8))]
    password: String,
    admin: Option<String>,
    enabled: Option<String>,
    limit: Option<i64>,
    teams: Option<HashSet<Key<Team>>>,
}

#[handler]
pub async fn post_new(
    env: Data<&Env>,
    next_token: &CsrfToken,
    verifier: &CsrfVerifier,
    SessionAdmin(auth): SessionAdmin,
    Form(form): Form<NewUserForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&form.token) {
        tracing::error!("Invalid CSRF token in new user form");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let mut errors = ValidationErrors::new();

    if let Err(first_errors) = form.validate() {
        errors.merge(first_errors);
    }

    if let Err(slug_error) = crate::utils::validate_slug(&form.username) {
        errors.add("username", slug_error);
    }

    if User::username_exists(&env.pool, None, &form.username)
        .await
        .map_err(InternalServerError)?
    {
        errors.add(
            "username",
            ValidationError::new("duplicate_username")
                .with_message("A user with this username already exists".into()),
        );
    }

    if !errors.is_empty() {
        let teams = TeamSelect::get(&env.pool).await.map_err(|err| {
            tracing::error!(?err, "Failed to get team selection");
            InternalServerError(err)
        })?;

        return Ok(render_template(
            "admin/users/form.html",
            context! {
                errors,
                teams,
                token => next_token.0,
                form => context! {
                    username => &form.username,
                    name => &form.name,
                    password => &form.password,
                    admin => form.admin.as_deref() == Some("on"),
                    enabled => form.enabled.as_deref() == Some("on"),
                    limit => form.limit,
                    teams => form.teams.unwrap_or_default(),
                },
                ..authorized_context(&env, &auth)
            },
        )?
        .with_header("HX-Retarget", "#user-form")
        .with_header("HX-Reselect", "#user-form")
        .into_response());
    }

    let NewUserForm {
        username,
        name,
        password,
        admin,
        enabled,
        limit,
        teams,
        ..
    } = form;

    let admin = admin.as_deref() == Some("on");
    let enabled = enabled.as_deref() == Some("on");
    let teams = teams.unwrap_or_default();

    let user = User {
        id: Key::new(),
        username,
        name,
        password: hash_password(&password),
        totp: None,
        enabled,
        admin,
        limit: limit.map(|limit| limit * 1024 * 1024),
        created_at: OffsetDateTime::now_utc(),
        created_by: Some(auth.id),
    };

    user.create(&env.pool).await.map_err(|err| {
        tracing::error!(?user, ?err, "Failed to create new user");
        InternalServerError(err)
    })?;

    tracing::info!(user = %user.id, username = ?user.username, "Created new user");

    for team in teams {
        tracing::info!(team_id = %team, user_id = %user.id, "Adding user to team");
        user.join_team(&env.pool, team).await.map_err(|err| {
            tracing::error!(err = ?err, user_id = %user.id, team_id = %team,
                            "Failed to add user to team");
            InternalServerError(err)
        })?;
    }

    Ok(Redirect::see_other("/admin/users").into_response())
}

#[derive(Debug, Deserialize)]
pub struct CheckUsernameForm {
    token: String,
    username: String,
}

#[handler]
pub async fn post_new_username(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    SessionAdmin(_): SessionAdmin,
    Form(CheckUsernameForm { token, username }): Form<CheckUsernameForm>,
) -> poem::Result<Html<String>> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in new username form");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let username = username.trim().to_string();
    let exists = User::username_exists(&env.pool, None, &username)
        .await
        .map_err(InternalServerError)?;

    render_template(
        "admin/users/username.html",
        context! {
            exists, form => context! { username }
        },
    )
}

#[handler]
pub async fn get_user(
    env: Data<&Env>,
    token: &CsrfToken,
    Path(user_id): Path<Key<User>>,
    SessionAdmin(admin): SessionAdmin,
) -> poem::Result<Html<String>> {
    let Some(user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(?err, %user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%user_id, "Unrecognized user ID");
        return render_404("Unrecognized user ID");
    };

    let teams = TeamSelect::get(&env.pool).await.map_err(|err| {
        tracing::error!(?err, "Failed to get team selection");
        InternalServerError(err)
    })?;

    let membership = user.get_teams(&env.pool).await.map_err(|err| {
        tracing::error!(?err, user_id = %user_id, "Failed to get user's team membership");
        InternalServerError(err)
    })?;

    render_template(
        "admin/users/form.html",
        context! {
            token => token.0,
            user,
            teams,
            membership,
            ..authorized_context(&env, &admin)
        },
    )
}

#[derive(Debug, Deserialize, Validate)]
pub struct EditUserForm {
    token: String,
    #[validate(length(min = 3, max = 100))]
    username: String,
    #[validate(length(min = 3, max = 100))]
    name: String,
    admin: Option<String>,
    enabled: Option<String>,
    limit: Option<i64>,
    teams: Option<HashSet<Key<Team>>>,
}

#[handler]
pub async fn post_user(
    env: Data<&Env>,
    SessionAdmin(auth): SessionAdmin,
    Path(user_id): Path<Key<User>>,
    next_token: &CsrfToken,
    verifier: &CsrfVerifier,
    Form(form): Form<EditUserForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&form.token) {
        tracing::error!("Invalid CSRF token in edit user form");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(mut user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = %user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!(user_id = %user_id, "Unrecognized user ID");
        return Ok(render_404("Unrecognized user ID")?.into_response());
    };

    let mut errors = ValidationErrors::new();

    if let Err(first_errors) = form.validate() {
        errors.merge(first_errors);
    }

    if form.username != user.username {
        if let Err(slug_error) = crate::utils::validate_slug(&form.username) {
            errors.add("username", slug_error);
        }

        if User::username_exists(&env.pool, Some(user_id), &form.username)
            .await
            .map_err(InternalServerError)?
        {
            errors.add(
                "username",
                ValidationError::new("duplicate_username")
                    .with_message("A user with this username already exists".into()),
            );
        }
    }

    if !errors.is_empty() {
        let teams = TeamSelect::get(&env.pool).await.map_err(|err| {
            tracing::error!(?err, "Failed to get team selection");
            InternalServerError(err)
        })?;

        let membership = user.get_teams(&env.pool).await.map_err(|err| {
            tracing::error!(?err, user_id = %user_id, "Failed to get user's team membership");
            InternalServerError(err)
        })?;

        return Ok(render_template(
            "admin/users/form.html",
            context! {
                errors,
                teams,
                user,
                token => next_token.0,
                form => context! {
                    username => &form.username,
                    name => &form.name,
                    admin => form.admin.as_deref() == Some("on"),
                    enabled => form.enabled.as_deref() == Some("on"),
                    limit => form.limit,
                    teams => form.teams.unwrap_or_default(),
                },
                ..authorized_context(&env, &auth)
            },
        )?
        .with_header("HX-Retarget", "#user-form")
        .with_header("HX-Reselect", "#user-form")
        .into_response());
    }

    let EditUserForm {
        username,
        name,
        admin,
        enabled,
        limit,
        teams,
        ..
    } = form;

    let admin = admin.as_deref() == Some("on");
    let enabled = enabled.as_deref() == Some("on");
    let limit = limit.map(|limit| limit * 1024 * 1024);
    let teams = teams.unwrap_or_default();

    // Override the 'enabled' selection if the user being edited is the same as the admin
    let enabled = user.id == auth.id || enabled;

    user.update(&env.pool, &username, &name, admin, enabled, limit)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, user_id = %user_id, "Failed to update user");
            InternalServerError(err)
        })?;

    tracing::info!(
        user_id = %user_id, admin = admin, enabled = enabled, limit = ?limit,
        "Updated user"
    );

    let membership = user.get_teams(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = %user_id, "Failed to get user's team membership");
        InternalServerError(err)
    })?;

    // Now remove the user from any teams they are no longer a member of
    for team in membership.iter().copied() {
        if !teams.contains(&team) {
            tracing::info!(team_id = %team, user_id = %user_id, "Removing user from team");
            user.leave_team(&env.pool, team).await.map_err(|err| {
                tracing::error!(err = ?err, user_id = %user_id, team_id = %team,
                                "Failed to remove user from team");
                InternalServerError(err)
            })?;
        }
    }

    // And add the user to any new teams
    for team in teams {
        if !membership.contains(&team) {
            tracing::info!(team_id = %team, user_id = %user_id, "Adding user to team");
            user.join_team(&env.pool, team).await.map_err(|err| {
                tracing::error!(err = ?err, user_id = %user_id, team_id = %team,
                                "Failed to add user to team");
                InternalServerError(err)
            })?;
        }
    }

    Ok(Redirect::see_other("/admin/users").into_response())
}

#[handler]
pub async fn delete_user(
    env: Data<&Env>,
    SessionAdmin(_): SessionAdmin,
    Path(user_id): Path<Key<User>>,
) -> poem::Result<Redirect> {
    let Some(mut user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = %user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!(user_id = %user_id, "Unrecognized user ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let upload_slugs = Upload::delete_for_user(&env.pool, user_id)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, user_id = %user_id, "Failed to delete users uploads");
            InternalServerError(err)
        })?;

    for slug in upload_slugs {
        let path = env.cache_dir.join(&slug);
        tracing::info!(path = ?path, owner = %user_id, "Deleting cached upload");
        if let Err(err) = tokio::fs::remove_file(&path).await {
            tracing::error!(path = ?path, err = ?err, owner = %user_id, "Failed to delete cached upload");
        }
    }

    user.delete(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = %user_id, "Failed to delete user");
        InternalServerError(err)
    })?;

    tracing::info!(user_id = %user_id, "Deleted user");
    Ok(Redirect::see_other("/admin/users"))
}

#[handler]
pub async fn post_check_username(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    Path(user_id): Path<Key<User>>,
    SessionAdmin(_): SessionAdmin,
    Form(CheckUsernameForm { token, username }): Form<CheckUsernameForm>,
) -> poem::Result<Html<String>> {
    if !verifier.is_valid(&token) {
        tracing::error!("Invalid CSRF token in new username form");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = %user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return render_404("Unrecognized user ID");
    };

    let username = username.trim().to_string();
    let exists = User::username_exists(&env.pool, Some(user_id), &username)
        .await
        .map_err(InternalServerError)?;

    render_template(
        "admin/users/username.html",
        context! {
            exists, user, form => context! { username }
        },
    )
}

#[handler]
pub async fn post_disable_user(
    env: Data<&Env>,
    Path(user_id): Path<Key<User>>,
    SessionAdmin(_): SessionAdmin,
) -> poem::Result<Redirect> {
    let Some(mut user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = %user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    user.set_enabled(&env.pool, false).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = %user_id, "Failed to disable user");
        InternalServerError(err)
    })?;

    tracing::info!(user_id = %user_id, "Disabled user");
    Ok(Redirect::see_other("/admin/users"))
}

#[handler]
pub async fn post_enable_user(
    env: Data<&Env>,
    Path(user_id): Path<Key<User>>,
    SessionAdmin(_): SessionAdmin,
) -> poem::Result<Redirect> {
    let Some(mut user) = User::get(&env.pool, user_id).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = %user_id, "Failed to get user");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized user ID '{user_id}'");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    user.set_enabled(&env.pool, true).await.map_err(|err| {
        tracing::error!(err = ?err, user_id = %user_id, "Failed to enable user");
        InternalServerError(err)
    })?;

    tracing::info!(user_id = %user_id, "Enabled user");
    Ok(Redirect::see_other("/admin/users"))
}
