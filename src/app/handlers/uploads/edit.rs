use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path},
    IntoResponse, Response,
};
use serde::Deserialize;
use serde_json::json;
use time::Date;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::{
    app::{
        extractors::user::SessionUser,
        handlers::utils::{check_permission, get_upload_by_id},
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{
        password::StoredPassword,
        types::Key,
        upload::{Upload, UploadPermission},
    },
    utils::ValidationErrorsExt,
};

#[handler]
pub async fn get_edit(
    env: Data<&Env>,
    token: &CsrfToken,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
) -> poem::Result<Html<String>> {
    let upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::Edit).await?;

    render_template(
        "uploads/edit.html",
        context! {
            token => token.0,
            now => time::OffsetDateTime::now_utc(),
            upload,
            has_password => upload.password.is_some(),
            ..authorized_context(&env, &user)
        },
    )
    .await
}

#[derive(Debug, Deserialize)]
pub struct CheckSlugForm {
    token: String,
    custom_slug: String,
}

#[handler]
pub async fn post_check_slug(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
    Form(CheckSlugForm { token, custom_slug }): Form<CheckSlugForm>,
) -> poem::Result<Html<String>> {
    if !verifier.is_valid(&token) {
        tracing::error!("CSRF token is invalid in upload edit");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let mut upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::Edit).await?;

    let custom_slug = custom_slug.trim().to_string();
    let exists = if let Some(owner_user) = upload.owner_user {
        Upload::custom_slug_exists(&env.pool, owner_user, Some(id), &custom_slug).await
    } else if let Some(owner_team) = upload.owner_team {
        Upload::custom_team_slug_exists(&env.pool, owner_team, Some(id), &custom_slug).await
    } else {
        tracing::error!("Upload has no owner");
        return Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
    }
    .map_err(|err| {
        tracing::error!(upload = %id, ?err, "Unable to check if custom slug exists");
        InternalServerError(err)
    })?;

    render_template(
        "uploads/edit/slug.html",
        context! {
            upload,
            exists,
            custom_slug,
        },
    )
    .await
}

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");

#[derive(Debug, Deserialize, Validate)]
pub struct UploadEditForm {
    token: String,
    #[validate(length(min = 1, max = 255))]
    filename: String,
    public: Option<String>,
    limit: Option<i64>,
    #[serde(default, with = "iso8601_date::option")]
    expiry_date: Option<Date>,
    has_password: Option<String>,
    change_password: Option<String>,
    password: Option<String>,
    #[validate(length(min = 3, max = 100))]
    custom_slug: Option<String>,
}

#[handler]
pub async fn post_edit(
    env: Data<&Env>,
    next_token: &CsrfToken,
    verifier: &CsrfVerifier,
    SessionUser(user): SessionUser,
    Path(id): Path<Key<Upload>>,
    Form(form): Form<UploadEditForm>,
) -> poem::Result<Response> {
    if !verifier.is_valid(&form.token) {
        tracing::error!("CSRF token is invalid in upload edit");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let mut upload = get_upload_by_id(&env, id).await?;
    check_permission(&env, &upload, Some(&user), UploadPermission::Edit).await?;

    let mut errors = ValidationErrors::new();
    if let Err(form_errors) = form.validate() {
        errors.merge(form_errors);
    }

    if let Some(ref custom_slug) = form.custom_slug {
        if upload.custom_slug.as_ref() != Some(custom_slug) {
            let exists = if let Some(owner_user) = upload.owner_user {
                Upload::custom_slug_exists(&env.pool, owner_user, Some(id), custom_slug).await
            } else if let Some(owner_team) = upload.owner_team {
                Upload::custom_team_slug_exists(&env.pool, owner_team, Some(id), custom_slug).await
            } else {
                tracing::error!("Upload has no owner");
                return Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
            }
            .map_err(|err| {
                tracing::error!(upload = %id, ?err, "Unable to check if custom slug exists");
                InternalServerError(err)
            })?;

            if exists {
                errors.add(
                    "custom_slug",
                    ValidationError::new("duplicate_slug")
                        .with_message("An upload with this custom slug already exists".into()),
                );
            }
        }
    }

    if !errors.is_empty() {
        return Ok(render_template(
            "uploads/edit.html",
            context! {
                errors,
                token => next_token.0,
                now => time::OffsetDateTime::now_utc(),
                has_password => upload.password.is_some(),
                form => context!{
                    filename => &form.filename,
                    public => form.public.as_deref() == Some("on"),
                    limit => form.limit,
                    expiry_date => form.expiry_date,
                    has_password => form.has_password.as_deref() == Some("on"),
                    change_password => form.change_password.as_deref() == Some("on"),
                    password => &form.password,
                    custom_slug => &form.custom_slug,
                },
                upload,
                ..authorized_context(&env, &user)
            },
        )
        .await?
        .with_header("HX-Retarget", "#upload-form")
        .with_header("HX-Reselect", "#upload-form")
        .into_response());
    }

    let UploadEditForm {
        filename,
        public,
        limit,
        expiry_date,
        has_password,
        password,
        custom_slug,
        ..
    } = form;

    let public = public.as_deref() == Some("on");

    let remaining = if upload.limit == limit {
        upload.remaining.or(limit)
    } else {
        limit
    };

    let has_password = has_password.as_deref() == Some("on");
    if has_password {
        if let Some(ref password) = password {
            upload.password = Some(StoredPassword::new(password)?);
        } else if upload.password.is_none() {
            tracing::error!("Password is required but not provided");
            return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
        }
    } else {
        upload.password = None;
    }

    tracing::info!(
        upload = %id,
        filename = ?filename,
        limit = ?limit,
        remaining = ?remaining,
        expiry = ?expiry_date,
        has_password = ?has_password,
        new_password = password.is_some(),
        custom_slug = ?custom_slug,
        "Updating upload");

    upload.filename = filename;
    upload.public = public;
    upload.limit = limit;
    upload.remaining = remaining;
    upload.expiry_date = expiry_date;
    upload.custom_slug = custom_slug;

    upload.save(&env.pool).await.map_err(InternalServerError)?;

    Ok(Html("")
        .with_header(
            "HX-Trigger",
            json!({
                "parcelUploadChanged": id,
            })
            .to_string(),
        )
        .into_response())
}
