use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use time::{Date, OffsetDateTime};

#[derive(Debug, FromRow, Serialize)]
pub struct Upload {
    pub id: i32,
    pub slug: String,
    pub filename: String,
    pub size: i64,
    pub public: bool,
    pub downloads: i64,
    pub limit: Option<i64>,
    pub expiry_date: Option<Date>,
    pub uploaded_by: i32,
    pub uploaded_at: OffsetDateTime,
    pub remote_addr: Option<String>,
}

impl Upload {
    pub async fn create(&mut self, pool: &SqlitePool) -> sqlx::Result<()> {
        let result = sqlx::query_scalar::<_, i32>(
            "INSERT INTO uploads (slug, filename, size, public,
            downloads, \"limit\", expiry_date,
            uploaded_by, uploaded_at, remote_addr)
            VALUES ($1, $2, $3, $4,
                    0, $5, $6,
                    $7, $8, $9)
            RETURNING id",
        )
        .bind(&self.slug)
        .bind(&self.filename)
        .bind(self.size)
        .bind(self.public)
        .bind(self.limit)
        .bind(self.expiry_date)
        .bind(self.uploaded_by)
        .bind(self.uploaded_at)
        .bind(&self.remote_addr)
        .fetch_one(pool)
        .await?;

        self.id = result;
        Ok(())
    }

    pub async fn edit(
        &mut self,
        pool: &SqlitePool,
        filename: &str,
        public: bool,
        limit: Option<i64>,
        expiry: Option<Date>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE uploads SET 
                filename = $1,
                public = $2,
                \"limit\" = $3,
                expiry_date = $4
            WHERE id = $5",
        )
        .bind(filename)
        .bind(public)
        .bind(limit)
        .bind(expiry)
        .bind(self.id)
        .execute(pool)
        .await?;

        self.filename = filename.to_string();
        self.public = public;
        self.limit = limit;
        Ok(())
    }

    pub async fn set_public(&mut self, pool: &SqlitePool, public: bool) -> sqlx::Result<()> {
        if public == self.public {
            return Ok(());
        }

        sqlx::query("UPDATE uploads SET public = $1 WHERE id = $2")
            .bind(public)
            .bind(self.id)
            .execute(pool)
            .await?;

        self.public = public;
        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: i32) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn get_for_user(pool: &SqlitePool, owner: i32) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE uploaded_by = $1 ORDER BY uploaded_at DESC")
            .bind(owner)
            .fetch_all(pool)
            .await
    }

    pub async fn get_by_slug(pool: &SqlitePool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }

    pub async fn record_download(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query("UPDATE uploads SET downloads = downloads + 1 WHERE id = ?")
            .bind(self.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM uploads WHERE id = ?")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete_for_user(pool: &SqlitePool, owner: i32) -> sqlx::Result<Vec<String>> {
        sqlx::query_scalar("DELETE FROM uploads WHERE uploaded_by = $1 RETURNING slug")
            .bind(owner)
            .fetch_all(pool)
            .await
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct UploadStats {
    pub total: i32,
    pub public: i32,
    pub downloads: i64,
    pub size: i64,
}

impl UploadStats {
    pub async fn get(pool: &SqlitePool) -> sqlx::Result<UploadStats> {
        sqlx::query_as(
            "SELECT COUNT(*) AS total, COUNT(public) AS public,
            SUM(downloads) AS downloads, SUM(size) AS size
            FROM uploads",
        )
        .fetch_one(pool)
        .await
    }

    pub async fn get_for(pool: &SqlitePool, owner: i32) -> sqlx::Result<UploadStats> {
        sqlx::query_as(
            "SELECT COUNT(*) AS total, COUNT(public) AS public,
            SUM(downloads) AS downloads, SUM(size) AS size
            FROM uploads
            WHERE uploaded_by = ?",
        )
        .bind(owner)
        .fetch_one(pool)
        .await
    }
}
