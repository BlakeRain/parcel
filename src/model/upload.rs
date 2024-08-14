use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, SqlitePool};
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
    pub remaining: Option<i64>,
    pub expiry_date: Option<Date>,
    #[serde(skip)]
    pub password: Option<String>,
    pub custom_slug: Option<String>,
    pub uploaded_by: i32,
    pub uploaded_at: OffsetDateTime,
    pub remote_addr: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UploadOrder {
    #[serde(rename = "filename")]
    Filename,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "downloads")]
    Downloads,
    #[serde(rename = "expiry_date")]
    ExpiryDate,
    #[serde(rename = "uploaded_at")]
    UploadedAt,
}

impl Default for UploadOrder {
    fn default() -> Self {
        Self::UploadedAt
    }
}

impl UploadOrder {
    fn get_order_field(&self) -> &'static str {
        match self {
            Self::Filename => "filename",
            Self::Size => "size",
            Self::Downloads => "downloads",
            Self::ExpiryDate => "expiry_date",
            Self::UploadedAt => "uploaded_at",
        }
    }
}

impl Upload {
    pub async fn create(&mut self, pool: &SqlitePool) -> sqlx::Result<()> {
        let result = sqlx::query_scalar::<_, i32>(
            "INSERT INTO uploads (slug, filename, size, public,
            downloads, \"limit\", remaining, expiry_date, password,
            custom_slug, uploaded_by, uploaded_at, remote_addr)
            VALUES ($1, $2, $3, $4,
                    0, $5, $6, $7, $8,
                    $9, $10, $11, $12)
            RETURNING id",
        )
        .bind(&self.slug)
        .bind(&self.filename)
        .bind(self.size)
        .bind(self.public)
        .bind(self.limit)
        .bind(self.remaining)
        .bind(self.expiry_date)
        .bind(&self.password)
        .bind(&self.custom_slug)
        .bind(self.uploaded_by)
        .bind(self.uploaded_at)
        .bind(&self.remote_addr)
        .fetch_one(pool)
        .await?;

        self.id = result;
        Ok(())
    }

    pub async fn save(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        let count = sqlx::query(
            "UPDATE uploads SET
            filename = $1,
            size = $2,
            public = $3,
            downloads = $4,
            \"limit\" = $5,
            remaining = $6,
            expiry_date = $7,
            password = $8,
            custom_slug = $9
            WHERE id = $10",
        )
        .bind(&self.filename)
        .bind(self.size)
        .bind(self.public)
        .bind(self.downloads)
        .bind(self.limit)
        .bind(self.remaining)
        .bind(self.expiry_date)
        .bind(&self.password)
        .bind(&self.custom_slug)
        .bind(self.id)
        .execute(pool)
        .await?;

        if count.rows_affected() == 0 {
            Err(sqlx::Error::RowNotFound)
        } else {
            Ok(())
        }
    }

    pub async fn set_public(&mut self, pool: &SqlitePool, public: bool) -> sqlx::Result<()> {
        if public == self.public {
            return Ok(());
        }

        let count = sqlx::query("UPDATE uploads SET public = $1 WHERE id = $2")
            .bind(public)
            .bind(self.id)
            .execute(pool)
            .await?;

        if count.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        self.public = public;
        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: i32) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn get_for_user(
        pool: &SqlitePool,
        owner: i32,
        order: UploadOrder,
        asc: bool,
        offset: u32,
        limit: u32,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(&format!(
            "SELECT * FROM uploads WHERE uploaded_by = $1 ORDER BY {} {} LIMIT {} OFFSET {}",
            order.get_order_field(),
            if asc { "ASC" } else { "DESC" },
            limit,
            offset,
        ))
        .bind(owner)
        .fetch_all(pool)
        .await
    }

    pub async fn count_for_user(pool: &SqlitePool, owner: i32) -> sqlx::Result<u32> {
        sqlx::query_scalar("SELECT COUNT(*) FROM uploads WHERE uploaded_by = $1")
            .bind(owner)
            .fetch_one(pool)
            .await
    }

    pub async fn get_by_slug(pool: &SqlitePool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }

    pub async fn get_by_custom_slug(
        pool: &SqlitePool,
        owner: i32,
        custom_slug: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE uploaded_by = $1 AND custom_slug = $2")
            .bind(owner)
            .bind(custom_slug)
            .fetch_optional(pool)
            .await
    }

    pub async fn custom_slug_exists(
        pool: &SqlitePool,
        owner: i32,
        existing: Option<i32>,
        custom_slug: &str,
    ) -> sqlx::Result<bool> {
        let mut query =
            QueryBuilder::new("SELECT EXISTS(SELECT 1 FROM uploads WHERE uploaded_by = ");

        query.push_bind(owner);
        query.push(" AND custom_slug = ");
        query.push_bind(custom_slug);

        if let Some(existing) = existing {
            query.push(" AND id != ");
            query.push_bind(existing);
        }

        query.push(")");

        query.build_query_scalar().fetch_one(pool).await
    }

    pub async fn record_download(&mut self, pool: &SqlitePool, public: bool) -> sqlx::Result<()> {
        let mut query = QueryBuilder::new("UPDATE uploads SET downloads = downloads + 1");

        if public && self.remaining.is_some() {
            query.push(", remaining = MAX(0, remaining - 1)");
        }

        query.push(" WHERE id = $1 RETURNING downloads, remaining");

        let (downloads, remaining) = query
            .build_query_as::<(i64, Option<i64>)>()
            .bind(self.id)
            .fetch_one(pool)
            .await?;

        self.downloads = downloads;
        self.remaining = remaining;

        Ok(())
    }

    pub async fn reset_remaining(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        let count = sqlx::query("UPDATE uploads SET remaining = \"limit\" WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        if count.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    pub async fn delete(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        let count = sqlx::query("DELETE FROM uploads WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        if count.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

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
            WHERE uploaded_by = $1",
        )
        .bind(owner)
        .fetch_one(pool)
        .await
    }
}
