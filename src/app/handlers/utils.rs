use poem::{error::InternalServerError, http::StatusCode};

use crate::{
    env::Env,
    model::{types::Key, upload::Upload, user::User},
};

pub async fn get_upload_by_id(env: &Env, id: Key<Upload>) -> poem::Result<Upload> {
    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(?err, %id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!(%id, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    Ok(upload)
}

pub async fn get_upload_by_slug(env: &Env, slug: &str) -> poem::Result<Upload> {
    let Some(upload) = Upload::get_by_slug(&env.pool, slug).await.map_err(|err| {
        tracing::error!(?err, ?slug, "Unable to get upload by slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    Ok(upload)
}

pub async fn check_owns_upload(env: &Env, user: &User, upload: &Upload) -> poem::Result<()> {
    let owner = user.admin || upload.is_owner(&env.pool, user).await.map_err(|err| {
        tracing::error!(%upload.id, %user.id, ?err, "Unable to check if user is owner of an upload");
        InternalServerError(err)
    })?;

    if !owner {
        tracing::error!(%user.id, %upload.id, "User is not the owner");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    Ok(())
}
