use std::{path::PathBuf, str::FromStr, sync::Arc};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};

use crate::{args::Args, model::migration::MIGRATOR};

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
    pub cache_dir: PathBuf,
    pub analytics_domain: Option<String>,
    pub plausible_script: Option<String>,
}

impl Env {
    pub async fn new(
        Args {
            db,
            cache_dir,
            analytics_domain,
            plausible_script,
            ..
        }: &Args,
    ) -> sqlx::Result<Self> {
        let cache_dir = cache_dir.clone();
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)?;
        }

        let opts = SqliteConnectOptions::from_str(db)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .pragma("synchronous", "normal")
            .pragma("journal_size_limit", "6144000")
            .pragma("mmap_size", "268435456");
        let pool = SqlitePool::connect_with(opts).await?;
        MIGRATOR.run(&pool).await?;

        let analytics_domain = analytics_domain.clone();
        let plausible_script = plausible_script.clone();
        let inner = Inner {
            pool,
            cache_dir,
            analytics_domain,
            plausible_script,
        };
        let inner = Arc::new(inner);

        Ok(Self { inner })
    }
}
