use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, SqlitePool};
use time::{Date, OffsetDateTime};

use super::{team::Team, types::Key, user::User};

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Upload {
    pub id: Key<Upload>,
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
    pub owner_team: Option<Key<Team>>,
    pub owner_user: Option<Key<User>>,
    pub uploaded_by: Key<User>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UploadPermission {
    View,
    Share,
    Download { with_password: bool },
    ResetDownloads,
    Edit,
    Transfer,
    Delete,
}

impl Upload {
    pub async fn create(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO uploads (id, slug, filename, size, public,
            downloads, \"limit\", remaining, expiry_date, password,
            custom_slug, uploaded_by, uploaded_at, remote_addr,
            owner_team, owner_user)
            VALUES ($1, $2, $3, $4, $5,
                    0, $6, $7, $8, $9,
                    $10, $11, $12, $13,
                    $14, $15)
            RETURNING id",
        )
        .bind(self.id)
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
        .bind(self.owner_team)
        .bind(self.owner_user)
        .execute(pool)
        .await?;

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
            custom_slug = $9,
            owner_team = $10,
            owner_user = $11
            WHERE id = $12",
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
        .bind(self.owner_team)
        .bind(self.owner_user)
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

    pub async fn get(pool: &SqlitePool, id: Key<Upload>) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
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
        owner: &str,
        custom_slug: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            "SELECT uploads.* FROM uploads \
            LEFT JOIN users ON uploads.owner_user = users.id \
            LEFT JOIN teams ON uploads.owner_team = teams.id \
            WHERE (users.username = $1 OR teams.slug = $1) AND uploads.custom_slug = $2",
        )
        .bind(owner)
        .bind(custom_slug)
        .fetch_optional(pool)
        .await
    }

    pub async fn custom_slug_exists(
        pool: &SqlitePool,
        owner: Key<User>,
        existing: Option<Key<Upload>>,
        custom_slug: &str,
    ) -> sqlx::Result<bool> {
        let mut query =
            QueryBuilder::new("SELECT EXISTS(SELECT 1 FROM uploads WHERE owner_user = ");

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

    pub async fn custom_team_slug_exists(
        pool: &SqlitePool,
        owner: Key<Team>,
        existing: Option<Key<Upload>>,
        custom_slug: &str,
    ) -> sqlx::Result<bool> {
        let mut query =
            QueryBuilder::new("SELECT EXISTS(SELECT 1 FROM uploads WHERE owner_team = ");

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

    pub async fn record_download(
        &mut self,
        pool: &SqlitePool,
        user: Option<&User>,
    ) -> sqlx::Result<()> {
        let mut query = QueryBuilder::new("UPDATE uploads SET downloads = downloads + 1");

        let public = match user {
            None => false,
            Some(user) => self.is_owner(pool, user).await?,
        };

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

    pub async fn delete_for_user(pool: &SqlitePool, owner: Key<User>) -> sqlx::Result<Vec<String>> {
        sqlx::query_scalar("DELETE FROM uploads WHERE owner_user = $1 RETURNING slug")
            .bind(owner)
            .fetch_all(pool)
            .await
    }

    pub async fn delete_for_team(pool: &SqlitePool, owner: Key<Team>) -> sqlx::Result<Vec<String>> {
        sqlx::query_scalar("DELETE FROM uploads WHERE owner_team = $1 RETURNING slug")
            .bind(owner)
            .fetch_all(pool)
            .await
    }

    pub async fn is_owner(&self, pool: &SqlitePool, user: &User) -> sqlx::Result<bool> {
        // If this upload is owned by a user, and we are passed that user.
        if matches!(self.owner_user, Some(owner) if owner == user.id) {
            return Ok(true);
        }

        // If this upload is owned by a team, check the user is a member of that team.
        if let Some(owner) = self.owner_team {
            return user.is_member_of(pool, owner).await;
        }

        Ok(false)
    }

    pub async fn find_teams_with_custom_slug_uploads(
        pool: &SqlitePool,
        user: Key<User>,
        custom_slug: &str,
    ) -> sqlx::Result<Vec<Key<Team>>> {
        sqlx::query_scalar(
            "SELECT teams.id \
            FROM uploads \
            LEFT JOIN teams ON teams.id = uploads.owner_team \
            LEFT JOIN team_members ON team_members.team = teams.id \
            WHERE team_members.user = $1 AND uploads.custom_slug = $2",
        )
        .bind(user)
        .bind(custom_slug)
        .fetch_all(pool)
        .await
    }

    pub async fn can_access(
        &self,
        pool: &SqlitePool,
        user: Option<&User>,
        permission: UploadPermission,
    ) -> sqlx::Result<bool> {
        if user.map(|user| user.admin).unwrap_or(false) {
            return Ok(true);
        }

        match permission {
            UploadPermission::View => {
                if self.public {
                    return Ok(true);
                }

                if let Some(user) = user {
                    if self.is_owner(pool, user).await? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            UploadPermission::Download { with_password } => {
                if self.public {
                    if let Some(remaining) = self.remaining {
                        if remaining < 1 {
                            return Ok(false);
                        }
                    }

                    if let Some(expiry) = self.expiry_date {
                        if expiry < OffsetDateTime::now_utc().date() {
                            return Ok(false);
                        }
                    }

                    if self.password.is_some() != with_password {
                        return Ok(false);
                    }

                    return Ok(true);
                }

                if let Some(user) = user {
                    if self.is_owner(pool, user).await? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            UploadPermission::Share
            | UploadPermission::Transfer
            | UploadPermission::ResetDownloads
            | UploadPermission::Edit
            | UploadPermission::Delete => {
                if let Some(user) = user {
                    self.is_owner(pool, user).await
                } else {
                    Ok(false)
                }
            }
        }
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

    pub async fn get_for_user(pool: &SqlitePool, owner: Key<User>) -> sqlx::Result<UploadStats> {
        sqlx::query_as(
            "SELECT COUNT(*) AS total, COUNT(public) AS public,
            SUM(downloads) AS downloads, SUM(size) AS size
            FROM uploads
            WHERE owner_user = $1",
        )
        .bind(owner)
        .fetch_one(pool)
        .await
    }

    pub async fn get_for_team(pool: &SqlitePool, owner: Key<Team>) -> sqlx::Result<UploadStats> {
        sqlx::query_as(
            "SELECT COUNT(*) AS total, COUNT(public) AS public,
            SUM(downloads) AS downloads, SUM(size) AS size
            FROM uploads
            WHERE owner_team = $1",
        )
        .bind(owner)
        .fetch_one(pool)
        .await
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct UploadList {
    pub id: Key<Upload>,
    pub slug: String,
    pub filename: String,
    pub size: i64,
    pub public: bool,
    pub downloads: i64,
    pub limit: Option<i64>,
    pub remaining: Option<i64>,
    pub expiry_date: Option<Date>,
    pub custom_slug: Option<String>,
    pub owner_slug: String,
    pub uploaded_by_id: Key<User>,
    pub uploaded_by_name: String,
    pub uploaded_at: OffsetDateTime,
}

impl UploadList {
    pub async fn get_for_user(
        pool: &SqlitePool,
        user: Key<User>,
        order: UploadOrder,
        asc: bool,
        offset: u32,
        limit: u32,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(&format!(
            "SELECT uploads.id, uploads.slug, uploads.filename, \
                uploads.size, uploads.public, uploads.downloads, \
                uploads.\"limit\", uploads.remaining, uploads.expiry_date, \
                uploads.custom_slug, \
                COALESCE(teams.slug, users.username) AS owner_slug, \
                uploads.uploaded_by AS uploaded_by_id, \
                uploader.name AS uploaded_by_name, \
                uploads.uploaded_at \
                FROM uploads \
                LEFT JOIN teams ON uploads.owner_team = teams.id \
                LEFT JOIN users ON uploads.owner_user = users.id \
                LEFT JOIN users AS uploader ON uploads.uploaded_by = uploader.id \
                WHERE uploads.owner_user = $1 \
                ORDER BY {} {} LIMIT {} OFFSET {}",
            order.get_order_field(),
            if asc { "ASC" } else { "DESC" },
            limit,
            offset
        ))
        .bind(user)
        .fetch_all(pool)
        .await
    }

    pub async fn get_for_team(
        pool: &SqlitePool,
        team: Key<Team>,
        order: UploadOrder,
        asc: bool,
        offset: u32,
        limit: u32,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(&format!(
            "SELECT uploads.id, uploads.slug, uploads.filename, \
                uploads.size, uploads.public, uploads.downloads, \
                uploads.\"limit\", uploads.remaining, uploads.expiry_date, \
                uploads.custom_slug, \
                COALESCE(teams.slug, users.username) AS owner_slug, \
                uploads.uploaded_by AS uploaded_by_id, \
                uploader.name AS uploaded_by_name, \
                uploads.uploaded_at \
                FROM uploads \
                LEFT JOIN teams ON uploads.owner_team = teams.id \
                LEFT JOIN users ON uploads.owner_user = users.id \
                LEFT JOIN users AS uploader ON uploads.uploaded_by = uploader.id \
                WHERE uploads.owner_team = $1 \
                ORDER BY {} {} LIMIT {} OFFSET {}",
            order.get_order_field(),
            if asc { "ASC" } else { "DESC" },
            limit,
            offset
        ))
        .bind(team)
        .fetch_all(pool)
        .await
    }
}
