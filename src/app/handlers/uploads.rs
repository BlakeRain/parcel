use poem::{
    error::InternalServerError,
    handler,
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH},
        StatusCode,
    },
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Multipart, Path, RealIp, Redirect},
    Body, Response,
};
use serde::Deserialize;
use time::{Date, OffsetDateTime};

use crate::{
    app::templates::{authorized_context, default_context, render_404, render_template},
    env::Env,
    model::{upload::Upload, user::User},
};

#[handler]
pub async fn get_uploads(env: Data<&Env>, user: User) -> poem::Result<Html<String>> {
    let uploads = Upload::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(user = user.id, err = ?err, "Unable to get uploads for user");
            InternalServerError(err)
        })?;

    let total: i64 = uploads.iter().map(|upload| upload.size).sum();

    let mut context = authorized_context(&user);
    context.insert("uploads", &uploads);
    context.insert("total", &total);
    render_template("uploads/list.html", &context)
}

#[handler]
pub fn get_new_upload(user: User) -> poem::Result<Html<String>> {
    let mut context = authorized_context(&user);
    render_template("uploads/new.html", &context)
}

#[handler]
pub async fn post_uploads(
    env: Data<&Env>,
    RealIp(ip): RealIp,
    user: User,
    mut form: Multipart,
) -> poem::Result<()> {
    // TODO: Other fields and CSRF token

    while let Ok(Some(field)) = form.next_field().await {
        if field.name() == Some("file") {
            let slug = nanoid::nanoid!();
            let filename = field
                .file_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| "unnamed.ext".to_string());

            let mut field = field.into_async_read();
            let path = env.cache_dir.join(&slug);

            {
                let mut file = tokio::fs::File::create(&path).await.map_err(|err| {
                    tracing::error!(err = ?err, path = ?path,
                                        "Unable to create file");
                    InternalServerError(err)
                })?;

                tokio::io::copy(&mut field, &mut file)
                    .await
                    .map_err(|err| {
                        tracing::error!(err = ?err, path = ?path,
                                        "Unable to copy file");
                        InternalServerError(err)
                    })?;
            }

            let meta = tokio::fs::metadata(&path).await.map_err(|err| {
                tracing::error!(err = ?err, path = ?path,
                                    "Unable to get metadata for file");
                InternalServerError(err)
            })?;

            let size = meta.len() as i64;
            tracing::info!(slug = slug, size = size, "Upload to cache complete");

            let mut upload = Upload {
                id: 0,
                slug,
                filename,
                size,
                public: false,
                downloads: 0,
                limit: None,
                expiry_date: None,
                uploaded_by: user.id,
                uploaded_at: OffsetDateTime::now_utc(),
                remote_addr: ip.as_ref().map(ToString::to_string),
            };

            upload.create(&env.pool).await.map_err(|err| {
                tracing::error!(err = ?err, "Unable to create upload");
                InternalServerError(err)
            })?;

            tracing::info!(upload = ?upload, "Created upload");
        } else {
            tracing::info!(field_name = field.name(), "Ignoring unrecognized field");
        }
    }

    Ok(())
}

#[handler]
pub async fn delete_uploads(
    env: Data<&Env>,
    user: User,
    Form(form): Form<Vec<(String, i32)>>,
) -> poem::Result<Redirect> {
    let ids = form
        .into_iter()
        .filter(|(name, _)| name == "selected")
        .map(|(_, id)| id)
        .collect::<Vec<_>>();

    for id in ids {
        let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
            tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
            InternalServerError(err)
        })?
        else {
            tracing::error!(id = ?id, "Unable to find upload with given ID");
            return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
        };

        if !user.admin && upload.uploaded_by != user.id {
            tracing::error!(
                user = user.id,
                upload = upload.id,
                "User tried to delete upload without permission"
            );

            return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
        }

        upload.delete(&env.pool).await.map_err(|err| {
            tracing::error!(err = ?err, upload = ?upload, "Unable to delete upload");
            InternalServerError(err)
        })?;

        let path = env.cache_dir.join(&upload.slug);
        tracing::info!(path = ?path, id = id, "Deleting cached upload");
        if let Err(err) = tokio::fs::remove_file(&path).await {
            tracing::error!(path = ?path, err = ?err, id = id, "Failed to delete cached upload");
        }
    }

    Ok(Redirect::see_other("/uploads"))
}

#[handler]
pub async fn get_upload(
    env: Data<&Env>,
    user: Option<User>,
    Path(slug): Path<String>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get_by_slug(&env.pool, &slug).await.map_err(|err| {
        tracing::error!(err = ?err, slug = ?slug, "Unable to get upload by slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(slug = ?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let owner = if let Some(user) = &user {
        user.admin || upload.uploaded_by == user.id
    } else {
        false
    };

    if !upload.public && !owner {
        tracing::error!(
            user = ?user,
            upload = ?upload,
            "User tried to access private upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    }

    let Some(uploader) = User::get(&env.pool, upload.uploaded_by)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, user_id = ?upload.uploaded_by,
                            "Unable to get user by ID");
            InternalServerError(err)
        })?
    else {
        tracing::error!(user_id = ?upload.uploaded_by, "Unable to find user with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let mut context = if let Some(user) = &user {
        authorized_context(user)
    } else {
        default_context()
    };

    let exhausted = if let Some(limit) = upload.limit {
        limit <= upload.downloads
    } else {
        false
    };

    let expired = if let Some(expiry) = upload.expiry_date {
        expiry < OffsetDateTime::now_utc().date()
    } else {
        false
    };

    context.insert("exhausted", &exhausted);
    context.insert("expired", &expired);
    context.insert("upload", &upload);
    context.insert("uploader", &uploader);

    context.insert("owner", &owner);
    render_template("uploads/view.html", &context)
}

#[handler]
pub async fn get_upload_download(
    env: Data<&Env>,
    user: Option<User>,
    Path(slug): Path<String>,
) -> poem::Result<Response> {
    let Some(upload) = Upload::get_by_slug(&env.pool, &slug).await.map_err(|err| {
        tracing::error!(err = ?err, slug = ?slug, "Unable to get upload by slug");
        InternalServerError(err)
    })?
    else {
        tracing::error!(slug = ?slug, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let owner = if let Some(user) = &user {
        user.admin || upload.uploaded_by == user.id
    } else {
        false
    };

    if !upload.public && !owner {
        tracing::error!(
            user = ?user,
            upload = ?upload,
            "User tried to access private upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    }

    if !owner {
        if let Some(limit) = upload.limit {
            if upload.downloads >= limit {
                tracing::error!(upload = ?upload, "Download limit was reached");
                return Err(poem::Error::from_status(StatusCode::GONE));
            }
        }

        if let Some(expiry) = upload.expiry_date {
            if expiry < OffsetDateTime::now_utc().date() {
                tracing::error!(upload = ?upload, "Upload has expired");
                return Err(poem::Error::from_status(StatusCode::GONE));
            }
        }
    }

    let path = env.cache_dir.join(&upload.slug);
    tracing::info!(upload = upload.id, path = ?path, "Opening file for upload");
    let file = tokio::fs::File::open(&path).await.map_err(|err| {
        tracing::error!(err = ?err, path = ?path, "Unable to open file");
        InternalServerError(err)
    })?;

    let meta = file.metadata().await.map_err(|err| {
        tracing::error!(err = ?err, path = ?path, "Unable to get metadata for file");
        InternalServerError(err)
    })?;

    upload.record_download(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, upload = ?upload, "Unable to record download");
        InternalServerError(err)
    })?;

    tracing::info!(upload = upload.id, meta = ?meta,
                   "Sending file to client");

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", upload.filename),
        )
        .header(CONTENT_LENGTH, meta.len())
        .body(Body::from_async_read(file)))
}

#[handler]
pub async fn delete_upload(
    env: Data<&Env>,
    user: User,
    Path(id): Path<i32>,
) -> poem::Result<Redirect> {
    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!(id = ?id, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            user = user.id,
            upload = upload.id,
            "User tried to delete upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    upload.delete(&env.pool).await.map_err(|err| {
        tracing::error!(err = ?err, upload = ?upload, "Unable to delete upload");
        InternalServerError(err)
    })?;

    let path = env.cache_dir.join(&upload.slug);
    tracing::info!(path = ?path, id = id, "Deleting cached upload");
    if let Err(err) = tokio::fs::remove_file(&path).await {
        tracing::error!(path = ?path, err = ?err, id = id, "Failed to delete cached upload");
    }

    Ok(Redirect::see_other("/uploads"))
}

#[handler]
pub async fn get_upload_edit(
    env: Data<&Env>,
    token: &CsrfToken,
    user: User,
    Path(id): Path<i32>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return render_404("Unrecognized upload ID");
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            user = user.id,
            upload = upload.id,
            "User tried to edit upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let mut context = authorized_context(&user);
    context.insert("token", &token.0);
    context.insert("upload", &upload);
    render_template("uploads/edit.html", &context)
}

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");

#[derive(Debug, Deserialize)]
pub struct UploadEditForm {
    token: String,
    filename: String,
    public: Option<String>,
    limit: Option<i64>,
    #[serde(default, with = "iso8601_date::option")]
    expiry_date: Option<Date>,
}

#[handler]
pub async fn put_upload_edit(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    user: User,
    Path(id): Path<i32>,
    Form(UploadEditForm {
        token,
        filename,
        public,
        limit,
        expiry_date,
    }): Form<UploadEditForm>,
) -> poem::Result<Redirect> {
    if !verifier.is_valid(&token) {
        tracing::error!("CSRF token is invalid in upload edit");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(mut upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            user = user.id,
            upload = upload.id,
            "User tried to edit upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let public = public.as_deref() == Some("on");

    tracing::info!(
        upload = id,
        filename = ?filename,
        limit = ?limit,
        expiry = ?expiry_date,
        "Updating upload");

    upload
        .edit(&env.pool, &filename, public, limit, expiry_date)
        .await
        .map_err(InternalServerError)?;

    Ok(Redirect::see_other("/uploads"))
}
