use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{Data, Html, Multipart, Path, RealIp},
};
use time::OffsetDateTime;

use crate::{
    app::templates::{authorized_context, default_context, render_template},
    env::Env,
    model::{upload::Upload, user::User},
};

#[handler]
pub fn get_new_upload(user: User) -> poem::Result<Html<String>> {
    let mut context = authorized_context(&user);
    render_template("new-upload.html", &context)
}

#[handler]
pub async fn post_new_upload(
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
                let mut file = tokio::fs::File::create(env.cache_dir.join(&slug))
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

    context.insert("upload", &upload);
    context.insert("uploader", &uploader);

    let owner = if let Some(user) = &user {
        user.admin || upload.uploaded_by == user.id
    } else {
        false
    };

    context.insert("owner", &owner);
    render_template("upload.html", &context)
}

#[handler]
pub async fn post_upload(Path(slug): Path<String>) -> poem::Result<()> {
    todo!()
}
