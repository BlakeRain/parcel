use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path, Query},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::{Date, OffsetDateTime};

use parcel_model::{types::Key, upload::Upload, user::User};

use crate::{
    app::{
        errors::CsrfError,
        extractors::admin::SessionAdmin,
        templates::{authorized_context, render_template},
    },
    env::Env,
};

#[derive(FromRow, Serialize)]
pub struct UploadListItem {
    pub id: Key<Upload>,
    pub slug: String,
    pub filename: String,
    pub size: i32,
    pub public: bool,
    pub downloads: i32,
    pub limit: Option<i32>,
    pub remaining: Option<i32>,
    pub expiry_date: Option<Date>,
    pub uploaded_by_id: Key<User>,
    pub uploaded_by_name: String,
    pub uploaded_at: OffsetDateTime,
    pub remote_addr: String,
}

#[handler]
pub async fn get_uploads(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
) -> poem::Result<Html<String>> {
    let uploads = sqlx::query_as::<_, UploadListItem>(
        "SELECT uploads.id, uploads.slug, uploads.filename, uploads.size, uploads.public,
                uploads.downloads, uploads.\"limit\", uploads.remaining, uploads.expiry_date,
                uploads.uploaded_by as uploaded_by_id,
                users.username as uploaded_by_name,
                uploads.uploaded_at, uploads.remote_addr
        FROM uploads
        LEFT OUTER JOIN users ON users.id = uploads.uploaded_by
        ORDER BY uploaded_at DESC
        LIMIT $1 OFFSET $2",
    )
    .bind(50 as i64)
    .bind(0 as i64)
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
            page => 0,
            ..authorized_context(&env, &admin)
        },
    )
    .await
}

#[handler]
pub async fn get_uploads_page(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
    Path(page): Path<u32>,
) -> poem::Result<Html<String>> {
    let uploads = sqlx::query_as::<_, UploadListItem>(
        "SELECT uploads.id, uploads.slug, uploads.filename, uploads.size, uploads.public,
                uploads.downloads, uploads.\"limit\", uploads.remaining, uploads.expiry_date,
                uploads.uploaded_by as uploaded_by_id,
                users.username as uploaded_by_name,
                uploads.uploaded_at, uploads.remote_addr
        FROM uploads
        LEFT OUTER JOIN users ON users.id = uploads.uploaded_by
        ORDER BY uploaded_at DESC
        LIMIT $1 OFFSET $2",
    )
    .bind(50 as i64)
    .bind((page * 50) as i64)
    .fetch_all(&env.pool)
    .await
    .map_err(|err| {
        tracing::error!(err = ?err, page = page, "Failed to fetch page of uploads");
        InternalServerError(err)
    })?;

    render_template(
        "admin/uploads/page.html",
        context! {
            uploads,
            page,
            ..authorized_context(&env, &admin)
        },
    )
    .await
}

#[handler]
pub async fn get_cache(
    env: Data<&Env>,
    csrf_token: &CsrfToken,
    SessionAdmin(admin): SessionAdmin,
) -> poem::Result<Html<String>> {
    render_template(
        "admin/uploads/cache.html",
        context! {
            csrf_token => csrf_token.0,
            ..authorized_context(&env, &admin)
        },
    )
    .await
}

trait WithCacheFiles: Default {
    #[allow(clippy::result_large_err)]
    fn valid_cache_file(&mut self, entry: std::fs::DirEntry) -> poem::Result<()>;
    #[allow(clippy::result_large_err)]
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
    fn valid_cache_file(&mut self, entry: std::fs::DirEntry) -> poem::Result<()> {
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
    fn valid_cache_file(&mut self, _entry: std::fs::DirEntry) -> poem::Result<()> {
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

    // First pass: collect all entries and their filenames
    let mut entries: Vec<(std::fs::DirEntry, String)> = Vec::new();
    let mut slugs: Vec<String> = Vec::new();

    for entry in dir {
        let entry = entry.map_err(|err| {
            tracing::error!(err = ?err, "Failed to read cache directory entry");
            InternalServerError(err)
        })?;

        let filename = entry.file_name().into_string().map_err(|filename| {
            tracing::error!(filename = ?filename, "Failed to convert filename to string");
            poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        // Extract base slug (remove .preview suffix if present)
        let slug = filename
            .strip_suffix(".preview")
            .unwrap_or(&filename)
            .to_string();

        slugs.push(slug.clone());
        entries.push((entry, slug));
    }

    // Batch fetch all existing slugs in a single query
    let existing_slugs = Upload::get_existing_slugs(&env.pool, &slugs)
        .await
        .map_err(|err| {
            tracing::error!(err = ?err, "Failed to fetch existing upload slugs");
            InternalServerError(err)
        })?;

    // Second pass: categorize entries based on HashSet membership
    for (entry, slug) in entries {
        if existing_slugs.contains(&slug) {
            result.valid_cache_file(entry)?;
        } else {
            result.invalid_cache_file(entry)?;
        }
    }

    Ok(result)
}

#[derive(Debug, Deserialize)]
pub struct CacheParams {
    csrf_token: String,
}

#[handler]
pub async fn post_cache(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
    next_token: &CsrfToken,
    csrf_verifier: &CsrfVerifier,
    Form(CacheParams { csrf_token }): Form<CacheParams>,
) -> poem::Result<Html<String>> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::error!("CSRF token is invalid in cache management");
        return Err(CsrfError.into());
    }

    let summary = find_cache_files::<CacheFilesSummary>(*env).await?;
    render_template(
        "admin/uploads/cache.html",
        context! {
            summary,
            csrf_token => next_token.0,
            ..authorized_context(&env, &admin)
        },
    )
    .await
}

#[handler]
pub async fn delete_cache(
    env: Data<&Env>,
    SessionAdmin(admin): SessionAdmin,
    next_token: &CsrfToken,
    csrf_verifier: &CsrfVerifier,
    Query(CacheParams { csrf_token }): Query<CacheParams>,
) -> poem::Result<Html<String>> {
    if !csrf_verifier.is_valid(&csrf_token) {
        tracing::error!("CSRF token is invalid in cache management");
        return Err(CsrfError.into());
    }

    let result = find_cache_files::<CacheFilesCleanup>(*env).await?;
    render_template(
        "admin/uploads/cache.html",
        context! {
            result,
            csrf_token => next_token.0,
            ..authorized_context(&env, &admin)
        },
    )
    .await
}
