use std::collections::HashMap;

use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    session::Session,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path, Query, Redirect},
    IntoResponse, Response,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use validator::{Validate, ValidationError, ValidationErrors};

use parcel_model::{
    password::StoredPassword,
    team::{Team, TeamMember, TeamSelect},
    types::Key,
    upload::{Upload, UploadOrder},
    user::{User, UserList},
};

use crate::{
    app::{
        extractors::admin::SessionAdmin,
        templates::{authorized_context, render_template},
    },
    env::Env,
    utils::{SessionExt, SizeUnit, ValidationErrorsExt},
};

#[handler]
pub async fn get_users(
    env: Data<&Env>,
    csrf_token: &CsrfToken,
    SessionAdmin(admin): SessionAdmin,
) -> poem::Result<Response> {
    let users = UserList::get(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, "Failed to get list of users");
        InternalServerError(err)
    })?;

    Ok(render_template(
        "admin/users.html",
        context! {
            users,
            csrf_token => csrf_token.0,
            ..authorized_context(&env, &admin)
        },
    )
    .await?
    .with_header("HX-Trigger", "closeModals")
    .into_response())
}

#[handler]
pub async fn get_new(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
    token: &CsrfToken,
) -> poem::Result<Html<String>> {
    let teams = Team::get_list(&env.pool).await.map_err(|err| {
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
    .await
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
    limit_unit: Option<SizeUnit>,
    teams: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TeamPermissionStruct {
    edit: bool,
    delete: bool,
    config: bool,
}

impl From<TeamMember> for TeamPermissionStruct {
    fn from(perm: TeamMember) -> Self {
        Self {
            edit: perm.can_edit,
            delete: perm.can_delete,
            config: perm.can_config,
        }
    }
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

    if Team::slug_exists(&env.pool, None, &form.username)
        .await
        .map_err(InternalServerError)?
    {
        errors.add(
            "username",
            ValidationError::new("duplicate_slug")
                .with_message("A team with this URL slug already exists".into()),
        );
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
                    limit_unit => form.limit_unit,
                    teams => form.teams,
                },
                ..authorized_context(&env, &auth)
            },
        )
        .await?
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
        limit_unit,
        teams,
        ..
    } = form;

    let admin = admin.as_deref() == Some("on");
    let enabled = enabled.as_deref() == Some("on");

    let teams: HashMap<Key<Team>, TeamPermissionStruct> =
        serde_json::from_str(&teams).map_err(|err| {
            tracing::error!(?err, "Failed to parse team permissions");
            InternalServerError(err)
        })?;

    let limit = limit.and_then(|limit| limit_unit.map(|unit| limit * unit.to_bytes()));

    let user = User {
        id: Key::new(),
        username,
        name,
        password: StoredPassword::new(&password)?,
        totp: None,
        enabled,
        admin,
        limit,
        created_at: OffsetDateTime::now_utc(),
        created_by: Some(auth.id),
        last_access: None,
        default_order: UploadOrder::UploadedAt,
        default_asc: false,
    };

    user.create(&env.pool).await.map_err(|err| {
        tracing::error!(user = %user.id, ?err, "Failed to create new user");
        InternalServerError(err)
    })?;

    tracing::info!(user = %user.id, username = ?user.username, "Created new user");

    for (team_id, permissions) in teams {
        tracing::info!(%team_id, user_id = %user.id, "Adding user to team");
        user.join_team(
            &env.pool,
            team_id,
            permissions.edit,
            permissions.delete,
            permissions.config,
        )
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, user_id = %user.id, %team_id,
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

    let user_exists = User::username_exists(&env.pool, None, &username)
        .await
        .map_err(InternalServerError)?;

    let team_exists = Team::slug_exists(&env.pool, None, &username)
        .await
        .map_err(InternalServerError)?;

    let exists = if user_exists {
        Some("user")
    } else if team_exists {
        Some("team")
    } else {
        None
    };

    render_template(
        "admin/users/username.html",
        context! {
            exists, form => context! { username }
        },
    )
    .await
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
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let teams = Team::get_list(&env.pool).await.map_err(|err| {
        tracing::error!(?err, "Failed to get team selection");
        InternalServerError(err)
    })?;

    let membership = TeamMember::get_for_user(&env.pool, user_id)
        .await
        .map_err(|err| {
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
    .await
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
    limit_unit: Option<SizeUnit>,
    teams: String,
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
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let mut errors = ValidationErrors::new();

    if let Err(first_errors) = form.validate() {
        errors.merge(first_errors);
    }

    if form.username != user.username {
        if let Err(slug_error) = crate::utils::validate_slug(&form.username) {
            errors.add("username", slug_error);
        }

        if Team::slug_exists(&env.pool, None, &form.username)
            .await
            .map_err(InternalServerError)?
        {
            errors.add(
                "username",
                ValidationError::new("duplicate_slug")
                    .with_message("A team with this URL slug already exists".into()),
            );
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
                membership,
                token => next_token.0,
                form => context! {
                    username => &form.username,
                    name => &form.name,
                    admin => form.admin.as_deref() == Some("on"),
                    enabled => form.enabled.as_deref() == Some("on"),
                    limit => form.limit,
                    limit_unit => form.limit_unit,
                    teams => form.teams,
                },
                ..authorized_context(&env, &auth)
            },
        )
        .await?
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
        limit_unit,
        teams,
        ..
    } = form;

    let admin = admin.as_deref() == Some("on");
    let enabled = enabled.as_deref() == Some("on");
    let limit = limit.and_then(|limit| limit_unit.map(|unit| limit * unit.to_bytes()));

    let teams: HashMap<Key<Team>, TeamPermissionStruct> =
        serde_json::from_str(&teams).map_err(|err| {
            tracing::error!(?err, "Failed to parse team permissions");
            InternalServerError(err)
        })?;

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

    // Remove the user from all their team memberships.
    for team in membership.iter().copied() {
        tracing::info!(team_id = %team, user_id = %user_id, "Removing user from team");
        user.leave_team(&env.pool, team).await.map_err(|err| {
            tracing::error!(err = ?err, user_id = %user_id, team_id = %team,
                        "Failed to remove user from team");
            InternalServerError(err)
        })?;
    }

    // Add the user to the teams they were selected for.
    for (team_id, permissions) in teams {
        tracing::info!(%team_id, user_id = %user_id, "Adding user to team");
        user.join_team(
            &env.pool,
            team_id,
            permissions.edit,
            permissions.delete,
            permissions.config,
        )
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, user_id = %user_id, %team_id,
                                "Failed to add user to team");
            InternalServerError(err)
        })?;
    }

    Ok(Redirect::see_other("/admin/users").into_response())
}

#[derive(Debug, Deserialize)]
pub struct DeleteUserQuery {
    csrf_token: String,
}

#[handler]
pub async fn delete_user(
    env: Data<&Env>,
    SessionAdmin(_): SessionAdmin,
    csrf_verifier: &CsrfVerifier,
    Path(user_id): Path<Key<User>>,
    Query(DeleteUserQuery { csrf_token }): Query<DeleteUserQuery>,
) -> poem::Result<Redirect> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::error!("Invalid CSRF token in delete user request");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

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
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let username = username.trim().to_string();

    let user_exists = User::username_exists(&env.pool, Some(user_id), &username)
        .await
        .map_err(InternalServerError)?;

    let team_exists = Team::slug_exists(&env.pool, None, &username)
        .await
        .map_err(InternalServerError)?;

    let exists = if user_exists {
        Some("user")
    } else if team_exists {
        Some("team")
    } else {
        None
    };

    render_template(
        "admin/users/username.html",
        context! {
            exists, user, form => context! { username }
        },
    )
    .await
}

#[derive(Debug, Deserialize)]
pub struct DisableUserForm {
    csrf_token: String,
}

#[handler]
pub async fn post_disable_user(
    env: Data<&Env>,
    SessionAdmin(_): SessionAdmin,
    csrf_verifier: &CsrfVerifier,
    Path(user_id): Path<Key<User>>,
    Form(DisableUserForm { csrf_token }): Form<DisableUserForm>,
) -> poem::Result<Redirect> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::error!("Invalid CSRF token in disable user request");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

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
    SessionAdmin(_): SessionAdmin,
    csrf_verifier: &CsrfVerifier,
    Path(user_id): Path<Key<User>>,
    Form(DisableUserForm { csrf_token }): Form<DisableUserForm>,
) -> poem::Result<Redirect> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::error!("Invalid CSRF token in disable user request");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

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

#[handler]
pub async fn get_masquerade(
    Path(user_id): Path<Key<User>>,
    SessionAdmin(admin): SessionAdmin,
    session: &Session,
) -> poem::Result<Redirect> {
    let mut stack = session
        .take::<Vec<Key<User>>>("masquerade_stack")
        .unwrap_or_default();
    stack.push(admin.id);
    session.set("masquerade_stack", stack);
    session.set("user_id", user_id);
    Ok(Redirect::see_other("/"))
}
