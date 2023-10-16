use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

#[derive(Debug, FromRow, Serialize)]
pub struct Upload {
    pub id: i32,
    pub slug: String,
    pub filename: String,
    pub public: bool,
    pub encrypted: bool,
    pub downloads: i32,
    pub remaining: Option<i32>,
    pub expiry_date: Option<OffsetDateTime>,
    pub uploaded_by: i32,
    pub uploaded_at: OffsetDateTime,
    pub remote_addr: String,
}

impl Upload {
    pub async fn create(&mut self, pool: &SqlitePool) -> sqlx::Result<()> {
        let result = sqlx::query_scalar::<_, i32>(
            "INSERT INTO uploads (slug, filename, public, encrypted,
            downloads, remaining, expiry_date,
            uploaded_by, uploaded_at, remote_addr)
            VALUES ($1, $2, $3, $4, 0, $5, $6, $7, $8, $9)
            RETURNING id",
        )
        .bind(&self.slug)
        .bind(&self.filename)
        .bind(self.public)
        .bind(self.encrypted)
        .bind(self.remaining)
        .bind(self.expiry_date)
        .bind(self.uploaded_by)
        .bind(&self.remote_addr)
        .fetch_one(pool)
        .await?;

        self.id = result;
        Ok(())
    }

    pub async fn get_by_slug(pool: &SqlitePool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }

    pub async fn record_download(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        if self.remaining.is_some() {
            sqlx::query(
                "UPDATE uploads SET downloads = downloads + 1,
                remaining = remaining - 1 WHERE id = ?",
            )
            .bind(self.id)
            .execute(pool)
            .await?;
        } else {
            sqlx::query("UPDATE uploads SET downloads = downloads + 1 WHERE id = ?")
                .bind(self.id)
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: i32) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM uploads WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }
}
