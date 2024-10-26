use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{team::Team, types::Key, upload::Upload},
};

#[handler]
pub async fn get_transfer(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_token: &CsrfToken,
    Path(upload_id): Path<Key<Upload>>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get(&env.pool, upload_id).await.map_err(|err| {
        tracing::error!(%user.id, %upload_id, %err, "Failed to get upload");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%user.id, %upload_id, "Upload not found");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    // Make sure that the user can access this upload.
    let is_owner = upload.is_owner(&env.pool, &user).await.map_err(|err| {
        tracing::error!(%user.id, %upload_id, %err, "Failed to check if user is owner");
        InternalServerError(err)
    })?;

    if !is_owner {
        tracing::error!(%user.id, %upload_id, "User is not the owner");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    // Get the teams for the user.
    let teams = Team::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, %upload_id, %err, "Failed to get teams");
            InternalServerError(err)
        })?;

    let teams_with_slugs = if let Some(ref custom_slug) = upload.custom_slug {
        // Look in the teams for this user for any uploads that have a matching custom slug.
        Upload::find_teams_with_custom_slug_uploads(&env.pool, user.id, custom_slug).await.map_err(|err| {
            tracing::error!(%user.id, %upload_id, %err, "Failed to find teams with custom slug uploads");
            InternalServerError(err)
        })?
    } else {
        vec![]
    };

    render_template(
        "uploads/transfer.html",
        context! {
            upload,
            teams,
            teams_with_slugs,
            csrf_token => csrf_token.0,
            ..authorized_context(&env, &user)
        },
    )
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferAction {
    Copy,
    Move,
}

#[derive(Debug, Deserialize)]
pub struct TransferForm {
    csrf_token: String,
    team: Key<Team>,
    action: TransferAction,
}

#[handler]
pub async fn post_transfer(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_verifier: &CsrfVerifier,
    Path(upload_id): Path<Key<Upload>>,
    Form(form): Form<TransferForm>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get(&env.pool, upload_id).await.map_err(|err| {
        tracing::error!(%user.id, %upload_id, %err, "Failed to get upload");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%user.id, %upload_id, "Upload not found");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    if !csrf_verifier.is_valid(&form.csrf_token) {
        tracing::error!(%user.id, %upload_id, "CSRF token verification failed");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    // Make sure that the user can access this upload.
    let is_owner = upload.is_owner(&env.pool, &user).await.map_err(|err| {
        tracing::error!(%user.id, %upload_id, %err, "Failed to check if user is owner");
        InternalServerError(err)
    })?;

    if !is_owner {
        tracing::error!(%user.id, %upload_id, "User is not the owner");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    let Some(team) = Team::get(&env.pool, form.team).await.map_err(|err| {
        tracing::error!(%user.id, %upload_id, %err, "Failed to get team");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%user.id, %upload_id, "Team not found");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    // Make sure that the user is a member of the team we're targetting.
    let is_member = user.is_member_of(&env.pool, team.id).await.map_err(|err| {
        tracing::error!(%user.id, %upload_id, %err, "Failed to check if user is a member of the team");
        InternalServerError(err)
    })?;

    if !is_member {
        tracing::error!(%user.id, %upload_id, "User is not a member of the team");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    // Make sure that the user is not trying to transfer the upload to a team that already has an
    // upload with the same custom slug.
    if let Some(ref custom_slug) = upload.custom_slug {
        let has_custom_slug = Upload::custom_team_slug_exists(&env.pool, team.id, None, custom_slug).await.map_err(|err| {
            tracing::error!(%user.id, %upload_id, %err, "Failed to check if team has custom slug upload");
            InternalServerError(err)
        })?;

        if has_custom_slug {
            tracing::error!(%user.id, %upload_id, "Team already has an upload with the same custom slug");
            return Err(poem::Error::from_status(StatusCode::CONFLICT));
        }
    }

    // Create a copy of the 'Upload' structure and give it a new ID and slug.
    let mut new_upload = upload.clone();
    new_upload.id = Key::new();
    new_upload.slug = nanoid::nanoid!();
    new_upload.owner_user = None;
    new_upload.owner_team = Some(team.id);

    // If we're moving the upload, then we can delete the old one from the database and then rename
    // the cache file to the new slug. Otherwise we'll copy the cache file to the new slug.
    if form.action == TransferAction::Move {
        if let Err(err) = tokio::fs::rename(
            env.cache_dir.join(&upload.slug),
            env.cache_dir.join(&new_upload.slug),
        )
        .await
        {
            tracing::error!(%user.id, %upload_id, %err, "Failed to move cache file");
            return Err(InternalServerError(err));
        }

        upload.delete(&env.pool).await.map_err(|err| {
            tracing::error!(%user.id, %upload_id, %err, "Failed to delete upload");
            InternalServerError(err)
        })?;
    } else if let Err(err) = tokio::fs::copy(
        env.cache_dir.join(&upload.slug),
        env.cache_dir.join(&new_upload.slug),
    )
    .await
    {
        tracing::error!(%user.id, %upload_id, %err, "Failed to copy cache file");
        return Err(InternalServerError(err));
    }

    // Save the new upload to the database.
    new_upload.create(&env.pool).await.map_err(|err| {
        tracing::error!(%user.id, %upload_id, %err, "Failed to save new upload");
        InternalServerError(err)
    })?;

    render_template(
        "uploads/transfer.html",
        context! {
            upload,
            new_upload,
            team,
            complete => true,
            action => form.action,
            ..authorized_context(&env, &user)
        },
    )
}
