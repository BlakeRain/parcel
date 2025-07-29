use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};

use parcel_model::migration::MIGRATOR;

use crate::args::Args;

pub struct Env {
    inner: Arc<Inner>,
}

impl Clone for Env {
    fn clone(&self) -> Self {
        let inner = Arc::clone(&self.inner);
        Self { inner }
    }
}

impl std::ops::Deref for Env {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct Inner {
    pub pool: SqlitePool,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub analytics_domain: Option<String>,
    pub plausible_script: Option<String>,

    /// The interval at which the preview generation worker checks for uploads to process.
    pub preview_generation_interval: Duration,

    /// The maximum size of an upload that can have a preview generated.
    ///
    /// If an upload is larger than this size, it will not have a preview generated. We don't
    /// record any `preview_error` though, as this is not really an error condition, and the user
    /// might change this value later.
    pub max_preview_size: Option<u64>,
}

impl Env {
    pub async fn new_with_pool(
        Args {
            config_dir,
            cache_dir,
            analytics_domain,
            plausible_script,
            preview_generation_interval,
            max_preview_size,
            ..
        }: &Args,
        pool: SqlitePool,
    ) -> sqlx::Result<Self> {
        let config_dir = config_dir.clone();
        if !config_dir.exists() {
            tracing::warn!("Config directory {config_dir:?} does not exist");
        }

        let cache_dir = cache_dir.clone();
        if !cache_dir.exists() {
            tracing::warn!("Cache directory {cache_dir:?} does not exist; creating it");
            std::fs::create_dir_all(&cache_dir)?;
        }

        let temp_dir = cache_dir.join("temp");
        if !temp_dir.exists() {
            tracing::warn!("Temporary directory {temp_dir:?} does not exist; creating it");
            std::fs::create_dir_all(&temp_dir)?;
        }

        let analytics_domain = analytics_domain.clone();
        let plausible_script = plausible_script.clone();
        let preview_generation_interval = Duration::from(*preview_generation_interval);
        let max_preview_size = *max_preview_size;
        let inner = Inner {
            pool,
            config_dir,
            cache_dir,
            analytics_domain,
            plausible_script,
            preview_generation_interval,
            max_preview_size,
        };
        let inner = Arc::new(inner);

        Ok(Self { inner })
    }

    pub async fn new(args: &Args) -> sqlx::Result<Self> {
        tracing::info!(db = ?args.db, "Creating SQLite connection pool");
        let opts = SqliteConnectOptions::from_str(&args.db)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .pragma("synchronous", "normal")
            .pragma("journal_size_limit", "6144000")
            .pragma("mmap_size", "268435456");
        let pool = SqlitePool::connect_with(opts).await?;

        tracing::info!("Running database migrations");
        MIGRATOR.run(&pool).await?;

        Self::new_with_pool(args, pool).await
    }
}
