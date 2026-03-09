use std::future::Future;

use anyhow::Context;

pub mod params;

pub use params::{ApplyParams, Params};

pub enum Connection {
    Sqlite(sqlx::SqlitePool),
    LibSql(libsql::Database),
}

impl From<sqlx::SqlitePool> for Connection {
    fn from(pool: sqlx::SqlitePool) -> Self {
        Self::Sqlite(pool)
    }
}

impl From<libsql::Database> for Connection {
    fn from(db: libsql::Database) -> Self {
        Self::LibSql(db)
    }
}

pub trait ConnectionExt {
    fn execute<P>(&self, query: &str, params: P) -> impl Future<Output = anyhow::Result<u64>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams;

    fn query_scalar<T, P>(&self, query: &str, params: P) -> impl Future<Output = anyhow::Result<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>;

    fn query_scalars<T, P>(
        &self,
        query: &str,
        params: P,
    ) -> impl Future<Output = anyhow::Result<Vec<T>>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>;

    fn query<T, P>(&self, query: &str, params: P) -> impl Future<Output = anyhow::Result<Vec<T>>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>;

    fn query_one<T, P>(&self, query: &str, params: P) -> impl Future<Output = anyhow::Result<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>;

    fn query_optional<T, P>(
        &self,
        query: &str,
        params: P,
    ) -> impl Future<Output = anyhow::Result<Option<T>>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>;
}

impl ConnectionExt for sqlx::SqlitePool {
    async fn execute<P>(&self, query: &str, params: P) -> anyhow::Result<u64>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
    {
        let result = params
            .apply_to(sqlx::query(query))
            .context("failed to bind parameeters")?
            .execute(self)
            .await
            .map_err(|err| anyhow::anyhow!(err))?;
        Ok(result.rows_affected())
    }

    async fn query_scalar<T, P>(&self, query: &str, params: P) -> anyhow::Result<T>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        params
            .apply_to(sqlx::query_scalar(query))
            .context("failed to bind parameters")?
            .fetch_one(self)
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    async fn query_scalars<T, P>(&self, query: &str, params: P) -> anyhow::Result<Vec<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        params
            .apply_to(sqlx::query_scalar(query))
            .context("failed to bind parameters")?
            .fetch_all(self)
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    async fn query<T, P>(&self, query: &str, params: P) -> anyhow::Result<Vec<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        params
            .apply_to(sqlx::query_as::<sqlx::Sqlite, T>(query))
            .context("failed to bind parameters")?
            .fetch_all(self)
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    async fn query_one<T, P>(&self, query: &str, params: P) -> anyhow::Result<T>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        params
            .apply_to(sqlx::query_as::<sqlx::Sqlite, T>(query))
            .context("failed to bind parameters")?
            .fetch_one(self)
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    async fn query_optional<T, P>(&self, query: &str, params: P) -> anyhow::Result<Option<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        params
            .apply_to(sqlx::query_as::<sqlx::Sqlite, T>(query))
            .context("failed to bind parameters")?
            .fetch_optional(self)
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }
}

impl ConnectionExt for libsql::Database {
    async fn execute<P>(&self, query: &str, params: P) -> anyhow::Result<u64>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
    {
        let conn = self.connect().context("failed to connect to database")?;
        conn.execute(query, params)
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    async fn query_scalar<T, P>(&self, query: &str, params: P) -> anyhow::Result<T>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        let conn = self.connect().context("failed to connect to database")?;
        let mut rows = conn
            .query(query, params)
            .await
            .context("failed to query database")?;

        let Some(row) = rows.next().await.context("failed to get first row")? else {
            return Err(anyhow::anyhow!("query did not return any rows"));
        };

        Ok(libsql::de::from_row::<(T,)>(&row)
            .context("failed to deserialize row")?
            .0)
    }

    async fn query_scalars<T, P>(&self, query: &str, params: P) -> anyhow::Result<Vec<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        let conn = self.connect().context("failed to connect to database")?;
        let mut rows = conn
            .query(query, params)
            .await
            .context("failed to query database")?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.context("failed to get first row")? {
            results.push(
                libsql::de::from_row::<(T,)>(&row)
                    .context("failed to deserialize row")?
                    .0,
            );
        }

        Ok(results)
    }

    async fn query<T, P>(&self, query: &str, params: P) -> anyhow::Result<Vec<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        let conn = self.connect().context("failed to connect to database")?;
        let mut rows = conn
            .query(query, params)
            .await
            .context("failed to query database")?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.context("failed to get first row")? {
            results.push(libsql::de::from_row::<T>(&row).context("failed to deserialize row")?);
        }

        Ok(results)
    }

    async fn query_one<T, P>(&self, query: &str, params: P) -> anyhow::Result<T>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        let conn = self.connect().context("failed to connect to database")?;
        let mut rows = conn
            .query(query, params)
            .await
            .context("failed to query database")?;

        let Some(row) = rows.next().await.context("failed to get first row")? else {
            return Err(anyhow::anyhow!("query did not return any rows"));
        };

        Ok(libsql::de::from_row::<T>(&row).context("failed to deserialize row")?)
    }

    async fn query_optional<T, P>(&self, query: &str, params: P) -> anyhow::Result<Option<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        let conn = self.connect().context("failed to connect to database")?;
        let mut rows = conn
            .query(query, params)
            .await
            .context("failed to query database")?;

        let Some(row) = rows.next().await.context("failed to get first row")? else {
            return Ok(None);
        };

        Ok(Some(
            libsql::de::from_row::<T>(&row).context("failed to deserialize row")?,
        ))
    }
}

impl ConnectionExt for Connection {
    async fn execute<P>(&self, query: &str, params: P) -> anyhow::Result<u64>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
    {
        match self {
            Self::Sqlite(pool) => pool.execute(query, params).await,
            Self::LibSql(db) => db.execute(query, params).await,
        }
    }

    async fn query_scalar<T, P>(&self, query: &str, params: P) -> anyhow::Result<T>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        match self {
            Self::Sqlite(pool) => pool.query_scalar(query, params).await,
            Self::LibSql(db) => db.query_scalar(query, params).await,
        }
    }

    async fn query_scalars<T, P>(&self, query: &str, params: P) -> anyhow::Result<Vec<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        match self {
            Self::Sqlite(pool) => pool.query_scalars(query, params).await,
            Self::LibSql(db) => db.query_scalars(query, params).await,
        }
    }

    async fn query<T, P>(&self, query: &str, params: P) -> anyhow::Result<Vec<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        match self {
            Self::Sqlite(pool) => pool.query(query, params).await,
            Self::LibSql(db) => db.query(query, params).await,
        }
    }

    async fn query_one<T, P>(&self, query: &str, params: P) -> anyhow::Result<T>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        match self {
            Self::Sqlite(pool) => pool.query_one(query, params).await,
            Self::LibSql(db) => db.query_one(query, params).await,
        }
    }

    async fn query_optional<T, P>(&self, query: &str, params: P) -> anyhow::Result<Option<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        match self {
            Self::Sqlite(pool) => pool.query_optional(query, params).await,
            Self::LibSql(db) => db.query_optional(query, params).await,
        }
    }
}
