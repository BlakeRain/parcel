use poem::{
    error::InternalServerError,
    handler,
    web::{Data, Html},
};
use serde::Serialize;
use sqlx::FromRow;
use time::{Date, OffsetDateTime};

use crate::{
    app::{
        extractors::admin::Admin,
        templates::{authorized_context, render_template},
    },
    env::Env,
};

#[derive(FromRow, Serialize)]
pub struct UploadListItem {
    pub id: i32,
    pub slug: String,
    pub filename: String,
    pub size: i32,
    pub public: bool,
    pub downloads: i32,
    pub limit: Option<i32>,
    pub expiry_date: Option<Date>,
    pub uploaded_by_id: i32,
    pub uploaded_by_name: String,
    pub uploaded_at: OffsetDateTime,
    pub remote_addr: String,
}

#[handler]
pub async fn get_uploads(env: Data<&Env>, Admin(admin): Admin) -> poem::Result<Html<String>> {
    let uploads = sqlx::query_as::<_, UploadListItem>(
        "SELECT uploads.id, uploads.slug, uploads.filename, uploads.size, uploads.public,
                uploads.downloads, uploads.\"limit\", uploads.expiry_date,
                uploads.uploaded_by as uploaded_by_id,
                users.username as uploaded_by_name,
                uploads.uploaded_at, uploads.remote_addr
        FROM uploads
        LEFT OUTER JOIN users ON users.id = uploads.uploaded_by
        ORDER BY uploaded_at DESC",
    )
    .fetch_all(&env.pool)
    .await
    .map_err(|err| {
        tracing::error!(err = ?err, "Failed to fetch list of uploads");
        InternalServerError(err)
    })?;

    let mut context = authorized_context(&env, &admin);
    context.insert("uploads", &uploads);
    render_template("admin/uploads.html", &context)
}
