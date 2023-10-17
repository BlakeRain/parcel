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
        .map_err(InternalServerError)?;

    let total: i32 = uploads.iter().map(|upload| upload.size).sum();

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
) -> poem::Result<String> {
    let slug = nanoid::nanoid!();
    let mut filename = None;
    let mut size = 0;
    let mut public = false;
    let mut limit = None;
    let mut expiry_date = None;

    while let Ok(Some(field)) = form.next_field().await {
        if field.name() == Some("filename") {
            filename = Some(field.text().await?);
        } else if field.name() == Some("file") {
            if let Some(name) = field.file_name() {
                if filename.is_none() {
                    filename = Some(name.to_string());
                }
            }

            let mut field = field.into_async_read();

            let path = env.cache_dir.join(&slug);

            {
                let mut file = tokio::fs::File::create(&path)
                    .await
                    .map_err(InternalServerError)?;
                tokio::io::copy(&mut field, &mut file)
                    .await
                    .map_err(InternalServerError)?;
            }

            let meta = tokio::fs::metadata(&path)
                .await
                .map_err(InternalServerError)?;
            size = meta.len() as i32;
        } else {
            tracing::info!(field_name = field.name(), "Ignoring unrecognized field");
        }
    }

    let filename = filename.unwrap_or_else(|| "unnamed".to_string());

    let mut upload = Upload {
        id: 0,
        slug: slug.clone(),
        filename,
        size,
        public,
        downloads: 0,
        limit,
        expiry_date,
        uploaded_by: user.id,
        uploaded_at: OffsetDateTime::now_utc(),
        remote_addr: ip.as_ref().map(ToString::to_string),
    };

    upload
        .create(&env.pool)
        .await
        .map_err(InternalServerError)?;

    Ok(slug)
}

#[handler]
pub async fn get_upload(
    env: Data<&Env>,
    user: Option<User>,
    Path(slug): Path<String>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get_by_slug(&env.pool, &slug)
        .await
        .map_err(InternalServerError)?
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
        .map_err(InternalServerError)?
    else {
        tracing::error!(user_id = ?upload.uploaded_by, "Unable to find user with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let mut context = if let Some(user) = &user {
        authorized_context(user)
    } else {
        default_context()
    };

    let expired = if let Some(expiry) = upload.expiry_date {
        expiry < OffsetDateTime::now_utc().date()
    } else {
        false
    };

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
    let Some(upload) = Upload::get_by_slug(&env.pool, &slug)
        .await
        .map_err(InternalServerError)?
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

    let path = env.cache_dir.join(&upload.slug);
    let file = tokio::fs::File::open(path)
        .await
        .map_err(InternalServerError)?;
    let meta = file.metadata().await.map_err(InternalServerError)?;

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
    let Some(upload) = Upload::get(&env.pool, id)
        .await
        .map_err(InternalServerError)?
    else {
        tracing::error!(id = ?id, "Unable to find upload with given ID");
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    if !user.admin && upload.uploaded_by != user.id {
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    upload
        .delete(&env.pool)
        .await
        .map_err(InternalServerError)?;

    Ok(Redirect::see_other("/uploads"))
}

#[handler]
pub async fn get_upload_edit(
    env: Data<&Env>,
    token: &CsrfToken,
    user: User,
    Path(id): Path<i32>,
) -> poem::Result<Html<String>> {
    let Some(upload) = Upload::get(&env.pool, id)
        .await
        .map_err(InternalServerError)?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return render_404("Unrecognized upload ID");
    };

    if !user.admin && upload.uploaded_by != user.id {
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
    limit: Option<i32>,
    #[serde(with = "iso8601_date::option")]
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
        tracing::error!("CSRF token is invalid");
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let Some(mut upload) = Upload::get(&env.pool, id)
        .await
        .map_err(InternalServerError)?
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
