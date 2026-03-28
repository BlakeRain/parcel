use minijinja::context;
use poem::{
    error::InternalServerError,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path},
};

use parcel_model::{
    tag::Tag,
    types::Key,
    upload::{Upload, UploadPermission},
};

use crate::{
    app::{
        errors::CsrfError,
        extractors::user::SessionUser,
        handlers::utils::{check_permission, get_upload_by_id, has_permission},
        templates::{authorized_context, render_template},
    },
    env::Env,
};

#[poem::handler]
pub async fn get_tags(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
) -> poem::Result<Html<String>> {
    let upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::View).await?;
    let editable = has_permission(&env, &upload, Some(&user), UploadPermission::Edit).await?;

    let tags = upload.get_tags(&env.pool).await.map_err(|err| {
        tracing::error!(?err, %id, "Failed to get tags for upload");
        InternalServerError(err)
    })?;

    render_template(
        "uploads/tags/tags.html",
        context! {
            upload,
            tags,
            editable,
            ..authorized_context(&env, &user)
        },
    )
    .await
}

#[poem::handler]
pub async fn get_tags_edit(
    env: Data<&Env>,
    csrf_token: &CsrfToken,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
) -> poem::Result<Html<String>> {
    let upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::Edit).await?;

    let tags = upload.get_tags(&env.pool).await.map_err(|err| {
        tracing::error!(?err, %id, "Failed to get tags for upload");
        InternalServerError(err)
    })?;

    let available = if let Some(owner_team) = upload.owner_team {
        Tag::get_for_team(&env.pool, owner_team)
            .await
            .map_err(|err| {
                tracing::error!(?err, %id, %owner_team, "Failed to get team owner of upload");
                InternalServerError(err)
            })?
    } else if let Some(owner_user) = upload.owner_user {
        Tag::get_for_user(&env.pool, owner_user)
            .await
            .map_err(|err| {
                tracing::error!(?err, %id, "Failed to get tags for user");
                InternalServerError(err)
            })?
    } else {
        vec![]
    };

    render_template(
        "uploads/tags/edit.html",
        context! {
            upload,
            tags,
            available,
            csrf_token => csrf_token.0,
            ..authorized_context(&env, &user)
        },
    )
    .await
}

#[poem::handler]
pub async fn post_tags_edit(
    env: Data<&Env>,
    csrf_verifier: &CsrfVerifier,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
    Form(form): Form<Vec<(String, String)>>,
) -> poem::Result<Html<String>> {
    let mut tags = Vec::new();
    let mut csrf_token = None;

    for (key, value) in form {
        if key == "csrf_token" {
            if csrf_token.is_some() {
                tracing::error!("Duplicate CSRF token in upload tags edit");
                return Err(CsrfError.into());
            }

            csrf_token = Some(value);
        } else if key == "tags" {
            tags.push(value);
        } else {
            tracing::error!(?key, "Unexpected form field in upload tags edit");
            return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
        }
    }

    let Some(csrf_token) = csrf_token else {
        tracing::error!("CSRF token not found in upload tags edit");
        return Err(CsrfError.into());
    };

    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::error!("CSRF token is invalid in upload tags edit");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::Edit).await?;
    let Some(owner) = upload.get_owner() else {
        tracing::error!(%id, "Upload has no owner (neither team nor user)");
        return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
    };

    // Find the corresponding IDs for each of the tags
    let tag_ids = {
        let mut tag_ids = Vec::new();

        for tag in &tags {
            let tag = Tag::get_or_create_for_owner(&env.pool, owner, tag)
                .await
                .map_err(|err| {
                    tracing::error!(?err, %id, ?tag, "Failed to get or create tag");
                    InternalServerError(err)
                })?;

            tag_ids.push(tag.id);
        }

        tag_ids
    };

    // Replace the tags attached to the upload.
    upload
        .replace_tags(&env.pool, tag_ids)
        .await
        .map_err(|err| {
            tracing::error!(?err, %id, "Failed to replace tags for upload");
            InternalServerError(err)
        })?;

    tags.sort();

    render_template(
        "uploads/tags/tags.html",
        context! {
            upload,
            tags,
            editable => true,
            ..authorized_context(&env, &user)
        },
    )
    .await
}
