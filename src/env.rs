use std::{path::PathBuf, str::FromStr, sync::Arc};

use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

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
}

impl Env {
    pub async fn new(Args { db, cache_dir, .. }: &Args) -> sqlx::Result<Self> {
        let cache_dir = cache_dir.clone();
        let opts = SqliteConnectOptions::from_str(db)?.create_if_missing(true);
        let pool = SqlitePool::connect_with(opts).await?;
        MIGRATOR.run(&pool).await?;

        let inner = Inner { pool, cache_dir };
        let inner = Arc::new(inner);

        Ok(Self { inner })
    }
}
