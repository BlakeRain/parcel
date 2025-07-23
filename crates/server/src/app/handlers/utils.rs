use poem::{error::InternalServerError, http::StatusCode};

use parcel_model::{
    types::Key,
    upload::{Upload, UploadPermission},
    user::User,
};

use crate::env::Env;

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

pub async fn check_permission(
    env: &Env,
    upload: &Upload,
    user: Option<&User>,
    permission: UploadPermission,
) -> poem::Result<()> {
    let granted = upload
        .can_access(&env.pool, user, permission)
        .await
        .map_err(|err| {
            tracing::error!(?err, upload = %upload.id, "Error checking upload permission");
            InternalServerError(err)
        })?;

    if !granted {
        let uid = user.map(|u| u.id);
        tracing::error!(upload = %upload.id, ?permission, user = ?uid,
            "User tried to access upload without permission");
        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    Ok(())
}
