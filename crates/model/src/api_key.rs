use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

use crate::{types::Key, user::User};

#[derive(Debug, FromRow, Serialize)]
pub struct ApiKey {
    pub id: Key<ApiKey>,
    pub owner: Key<User>,
    #[serde(skip)]
    pub code: String,
    pub name: String,
    pub enabled: bool,
    pub created_at: OffsetDateTime,
    pub created_by: Option<Key<User>>,
    pub last_used: Option<OffsetDateTime>,
}

impl ApiKey {
    pub fn new(owner: Key<User>, code: String, name: String, created_by: Key<User>) -> Self {
        let id = Key::new();
        Self {
            id,
            owner,
            code,
            name,
            enabled: true,
            created_at: OffsetDateTime::now_utc(),
            created_by: Some(created_by),
            last_used: None,
        }
    }

    pub async fn create(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO api_keys \
            (id, owner, code, name, enabled, created_at, created_by) \
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(self.id)
        .bind(self.owner)
        .bind(&self.code)
        .bind(&self.name)
        .bind(self.enabled)
        .bind(self.created_at)
        .bind(self.created_by)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: Key<ApiKey>) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM api_keys WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn get_by_code(pool: &SqlitePool, code: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM api_keys WHERE code = $1")
            .bind(code)
            .fetch_optional(pool)
            .await
    }

    pub async fn record_last_use(&mut self, pool: &SqlitePool) -> sqlx::Result<()> {
        let now = OffsetDateTime::now_utc();

        let result = sqlx::query("UPDATE api_keys SET last_used = $1 WHERE id = $2")
            .bind(now)
            .bind(self.id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        self.last_used = Some(now);
        Ok(())
    }
}
