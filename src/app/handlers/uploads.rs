use esbuild_bundle::javascript;
use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH},
        StatusCode,
    },
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Multipart, Path, Query, RealIp, Redirect},
    Body, IntoResponse, Response,
};
use serde::Deserialize;
use time::{Date, OffsetDateTime};

use crate::{
    app::templates::{authorized_context, default_context, render_404, render_template},
    env::Env,
    model::{
        upload::{Upload, UploadStats},
        user::User,
    },
};

#[handler]
pub async fn get_list(env: Data<&Env>, user: User) -> poem::Result<Html<String>> {
    let uploads = Upload::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(user = user.id, err = ?err, "Unable to get uploads for user");
            InternalServerError(err)
        })?;

    render_template(
        "uploads/list.html",
        context! {
            uploads,
            ..authorized_context(&env, &user)
        },
    )
}

#[handler]
pub async fn delete_list(
    env: Data<&Env>,
    user: User,
    Form(form): Form<Vec<(String, i32)>>,
) -> poem::Result<impl IntoResponse> {
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

    Ok(Html("").with_header("HX-Refresh", "true"))
}

#[handler]
pub async fn get_stats(env: Data<&Env>, user: User) -> poem::Result<Html<String>> {
    let stats = UploadStats::get_for(&env.pool, user.id)
        .await
        .map_err(InternalServerError)?;

    // Generate a random int between 10 and 90
    let random = rand::random::<i8>() % 80 + 10;

    render_template(
        "uploads/stats.html",
        context! {
            stats,
            random,
            ..authorized_context(&env, &user)
        },
    )
}

#[handler]
pub async fn get_new(
    env: Data<&Env>,
    csrf_token: &CsrfToken,
    user: User,
) -> poem::Result<Html<String>> {
    render_template(
        "uploads/new.html",
        context! {
            csrf_token => csrf_token.0,
            upload_js => javascript!("scripts/components/upload.ts"),
            ..authorized_context(&env, &user)
        },
    )
}

#[handler]
pub async fn post_new(
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

    render_template(
        "uploads/view.html",
        context! {
            exhausted,
            expired,
            upload,
            uploader,
            owner,
            ..if let Some(user) = &user {
                authorized_context(&env, user)
            } else {
                default_context(&env)
            }
        },
    )
}

#[handler]
pub async fn delete_upload(
    env: Data<&Env>,
    user: User,
    Path(id): Path<i32>,
) -> poem::Result<impl IntoResponse> {
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

    let remaining = Upload::count_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, "Failed to count remaining uploads for user");
            InternalServerError(err)
        })?;

    Ok(Html(if remaining == 0 {
        "<tr><td colspan=\"9\" class=\"text-center italic\">Nothing to show</td></tr>"
    } else {
        ""
    })
    .with_header("HX-Trigger", "parcelRefresh"))
}

#[derive(Debug, Deserialize)]
pub struct EditQuery {
    hx_target: Option<String>,
    ult_dest: Option<String>,
}

#[handler]
pub async fn get_edit(
    env: Data<&Env>,
    token: &CsrfToken,
    user: User,
    Path(id): Path<i32>,
    Query(EditQuery {
        hx_target,
        ult_dest,
    }): Query<EditQuery>,
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

    render_template(
        "uploads/edit.html",
        context! {
            token => token.0,
            upload,
            hx_target,
            ult_dest,
            ..authorized_context(&env, &user)
        },
    )
}

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");

#[derive(Debug, Deserialize)]
pub struct UploadEditForm {
    token: String,
    ult_dest: Option<String>,
    filename: String,
    public: Option<String>,
    limit: Option<i64>,
    #[serde(default, with = "iso8601_date::option")]
    expiry_date: Option<Date>,
}

#[handler]
pub async fn post_edit(
    env: Data<&Env>,
    verifier: &CsrfVerifier,
    user: User,
    Path(id): Path<i32>,
    Form(UploadEditForm {
        token,
        ult_dest,
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

    Ok(Redirect::see_other(
        ult_dest.unwrap_or_else(|| "/uploads/list".to_string()),
    ))
}

#[derive(Debug, Deserialize)]
pub struct MakePublicQuery {
    public: bool,
    ult_dest: Option<String>,
}

#[handler]
pub async fn post_public(
    env: Data<&Env>,
    user: User,
    Path(id): Path<i32>,
    Query(MakePublicQuery { public, ult_dest }): Query<MakePublicQuery>,
) -> poem::Result<Response> {
    let Some(mut upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return render_404("Unrecognized upload ID").map(IntoResponse::into_response);
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            user = user.id,
            upload = upload.id,
            "User tried to edit upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    tracing::info!(id = id, public = public, "Setting upload public state");
    upload
        .set_public(&env.pool, public)
        .await
        .map_err(InternalServerError)?;

    if let Some(ult_dest) = ult_dest {
        Ok(Redirect::see_other(ult_dest).into_response())
    } else {
        Ok(Html("").with_header("HX-Redirect", "/").into_response())
    }
}

#[handler]
pub async fn get_download(
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

    if !owner {
        upload.record_download(&env.pool).await.map_err(|err| {
            tracing::error!(err = ?err, upload = ?upload, "Unable to record download");
            InternalServerError(err)
        })?;
    }

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
