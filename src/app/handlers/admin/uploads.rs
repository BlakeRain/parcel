use minijinja::context;
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
    model::upload::Upload,
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

    render_template(
        "admin/uploads.html",
        context! {
            uploads,
            ..authorized_context(&env, &admin)
        },
    )
}

#[handler]
pub fn get_cache(env: Data<&Env>, Admin(admin): Admin) -> poem::Result<Html<String>> {
    render_template("admin/uploads/cache.html", authorized_context(&env, &admin))
}

trait WithCacheFiles: Default {
    fn valid_cache_file(&mut self, entry: std::fs::DirEntry, upload: Upload) -> poem::Result<()>;
    fn invalid_cache_file(&mut self, entry: std::fs::DirEntry) -> poem::Result<()>;
}

#[derive(Debug, Default, Serialize)]
struct CacheFilesSummary {
    #[serde(rename = "validTotal")]
    valid_total: u64,
    #[serde(rename = "validCount")]
    valid_count: u64,
    #[serde(rename = "invalidTotal")]
    invalid_total: u64,
    #[serde(rename = "invalidCount")]
    invalid_count: u64,
}

impl WithCacheFiles for CacheFilesSummary {
    fn valid_cache_file(&mut self, entry: std::fs::DirEntry, _upload: Upload) -> poem::Result<()> {
        self.valid_total += entry
            .metadata()
            .map_err(|err| {
                tracing::error!(err = ?err, "Failed to read cache directory entry metadata");
                InternalServerError(err)
            })?
            .len();

        self.valid_count += 1;
        Ok(())
    }

    fn invalid_cache_file(&mut self, entry: std::fs::DirEntry) -> poem::Result<()> {
        self.invalid_total += entry
            .metadata()
            .map_err(|err| {
                tracing::error!(err = ?err, "Failed to read cache directory entry metadata");
                InternalServerError(err)
            })?
            .len();

        self.invalid_count += 1;
        Ok(())
    }
}

#[derive(Debug, Default, Serialize)]
struct CacheFilesCleanup {
    #[serde(rename = "removedTotal")]
    removed_total: u64,
    #[serde(rename = "removedCount")]
    removed_count: u64,
}

impl WithCacheFiles for CacheFilesCleanup {
    fn valid_cache_file(&mut self, _entry: std::fs::DirEntry, _upload: Upload) -> poem::Result<()> {
        Ok(())
    }

    fn invalid_cache_file(&mut self, entry: std::fs::DirEntry) -> poem::Result<()> {
        self.removed_total += entry
            .metadata()
            .map_err(|err| {
                tracing::error!(err = ?err, "Failed to read cache directory entry metadata");
                InternalServerError(err)
            })?
            .len();

        std::fs::remove_file(entry.path()).map_err(|err| {
            tracing::error!(err = ?err, "Failed to remove cache file");
            InternalServerError(err)
        })?;

        self.removed_count += 1;
        Ok(())
    }
}

async fn find_cache_files<T>(env: &Env) -> poem::Result<T>
where
    T: WithCacheFiles,
{
    let mut result = T::default();

    let dir = std::fs::read_dir(&env.cache_dir).map_err(|err| {
        tracing::error!(err = ?err, dir = ?env.cache_dir, "Failed to read cache directory");
        InternalServerError(err)
    })?;

    for entry in dir {
        let entry = entry.map_err(|err| {
            tracing::error!(err = ?err, "Failed to read cache directory entry");
            InternalServerError(err)
        })?;

        let filename = entry.file_name().into_string().map_err(|filename| {
            tracing::error!(filename = ?filename, "Failed to convert filename to string");
            poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        let upload = Upload::get_by_slug(&env.pool, &filename)
            .await
            .map_err(|err| {
                tracing::error!(err = ?err, slug = ?filename, "Failed to fetch upload by slug");
                InternalServerError(err)
            })?;

        if let Some(upload) = upload {
            result.valid_cache_file(entry, upload)?;
        } else {
            result.invalid_cache_file(entry)?;
        }
    }

    Ok(result)
}

#[handler]
pub async fn post_cache(env: Data<&Env>, Admin(admin): Admin) -> poem::Result<Html<String>> {
    let summary = find_cache_files::<CacheFilesSummary>(*env).await?;
    render_template(
        "admin/uploads/cache.html",
        context! {
            summary,
            ..authorized_context(&env, &admin)
        },
    )
}

#[handler]
pub async fn delete_cache(env: Data<&Env>, Admin(admin): Admin) -> poem::Result<Html<String>> {
    let result = find_cache_files::<CacheFilesCleanup>(*env).await?;
    render_template(
        "admin/uploads/cache.html",
        context! {
            result,
            ..authorized_context(&env, &admin)
        },
    )
}
