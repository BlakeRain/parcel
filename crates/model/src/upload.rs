use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, SqlitePool};
use time::{Date, OffsetDateTime};

use super::{
    password::StoredPassword,
    tag::Tag,
    team::{Team, TeamMember},
    types::Key,
    user::User,
};

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
    pub password: Option<StoredPassword>,
    pub custom_slug: Option<String>,
    pub owner_team: Option<Key<Team>>,
    pub owner_user: Option<Key<User>>,
    pub uploaded_by: Option<Key<User>>,
    pub uploaded_at: OffsetDateTime,
    pub remote_addr: Option<String>,
    pub mime_type: Option<String>,
    pub has_preview: bool,
    pub preview_error: Option<String>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum UploadOrder {
    #[serde(rename = "filename")]
    Filename,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "downloads")]
    Downloads,
    #[serde(rename = "expiry_date")]
    ExpiryDate,
    #[default]
    #[serde(rename = "uploaded_at")]
    UploadedAt,
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

pub enum UploadOwnership {
    OwnedByUser,
    OwnedByTeam(TeamMember),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadOwner {
    User(Key<User>),
    Team(Key<Team>),
}

impl Upload {
    pub fn get_owner(&self) -> Option<UploadOwner> {
        if let Some(owner_user) = self.owner_user {
            return Some(UploadOwner::User(owner_user));
        }

        if let Some(owner_team) = self.owner_team {
            return Some(UploadOwner::Team(owner_team));
        }

        None
    }

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

    /// Fetch multiple uploads by their IDs in a single query.
    pub async fn get_many(pool: &SqlitePool, ids: &[Key<Upload>]) -> sqlx::Result<Vec<Self>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut query = QueryBuilder::new("SELECT * FROM uploads WHERE id IN (");
        let mut separated = query.separated(", ");
        for id in ids {
            separated.push_bind(*id);
        }
        separated.push_unseparated(")");

        query.build_query_as().fetch_all(pool).await
    }

    /// Delete multiple uploads by their IDs in a single query.
    /// Returns the slugs of the deleted uploads (for cache cleanup).
    pub async fn delete_many(pool: &SqlitePool, ids: &[Key<Upload>]) -> sqlx::Result<Vec<String>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut query = QueryBuilder::new("DELETE FROM uploads WHERE id IN (");
        let mut separated = query.separated(", ");
        for id in ids {
            separated.push_bind(*id);
        }
        separated.push_unseparated(") RETURNING slug");

        query.build_query_scalar().fetch_all(pool).await
    }

    pub async fn get_by_slug(pool: &SqlitePool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }

    /// Check which slugs exist in the database from a list of candidates.
    /// Returns only the slugs that exist.
    pub async fn get_existing_slugs(
        pool: &SqlitePool,
        slugs: &[String],
    ) -> sqlx::Result<std::collections::HashSet<String>> {
        if slugs.is_empty() {
            return Ok(std::collections::HashSet::new());
        }

        let mut query = QueryBuilder::new("SELECT slug FROM uploads WHERE slug IN (");
        let mut separated = query.separated(", ");
        for slug in slugs {
            separated.push_bind(slug);
        }
        separated.push_unseparated(")");

        let existing: Vec<String> = query.build_query_scalar().fetch_all(pool).await?;
        Ok(existing.into_iter().collect())
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
            Some(user) => self.is_owner(pool, user).await?.is_some(),
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

    pub async fn set_password(&mut self, pool: &SqlitePool, password: &str) -> anyhow::Result<()> {
        let password = StoredPassword::new(password).context("failed to hash password")?;

        let result = sqlx::query("UPDATE uploads SET password = $1 WHERE id = $2")
            .bind(&password)
            .bind(self.id)
            .execute(pool)
            .await
            .context("failed to update user password")?;

        if result.rows_affected() == 0 {
            anyhow::bail!("Upload not found");
        }

        self.password = Some(password);
        Ok(())
    }

    pub async fn set_mime_type(&mut self, pool: &SqlitePool, mime_type: &str) -> sqlx::Result<()> {
        let result = sqlx::query("UPDATE uploads SET mime_type = $1 WHERE id = $2")
            .bind(mime_type)
            .bind(self.id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        self.mime_type = Some(mime_type.to_string());
        Ok(())
    }

    pub async fn set_preview_error<E: Into<String>>(
        &mut self,
        pool: &SqlitePool,
        error: E,
    ) -> sqlx::Result<()> {
        let error = error.into();

        let result = sqlx::query("UPDATE uploads SET preview_error = $1 WHERE id = $2")
            .bind(&error)
            .bind(self.id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        self.preview_error = Some(error);
        Ok(())
    }

    pub async fn clear_preview_error(&mut self, pool: &SqlitePool) -> sqlx::Result<()> {
        let result = sqlx::query("UPDATE uploads SET preview_error = NULL WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        self.preview_error = None;
        Ok(())
    }

    pub async fn set_has_preview(
        &mut self,
        pool: &SqlitePool,
        has_preview: bool,
    ) -> sqlx::Result<()> {
        let result =
            sqlx::query("UPDATE uploads SET has_preview = $1, preview_error = $2 WHERE id = $3")
                .bind(has_preview)
                .bind(if has_preview {
                    None::<&String>
                } else {
                    self.preview_error.as_ref()
                })
                .bind(self.id)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        self.has_preview = has_preview;
        Ok(())
    }

    pub async fn get_all_without_preview(
        pool: &SqlitePool,
        offset: u32,
        limit: u32,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT * FROM uploads \
            WHERE NOT has_preview AND preview_error IS NULL \
            LIMIT $1 \
            OFFSET $2",
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(pool)
        .await
    }

    pub async fn delete(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        let result = sqlx::query("DELETE FROM uploads WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
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

    pub async fn is_owner(
        &self,
        pool: &SqlitePool,
        user: &User,
    ) -> sqlx::Result<Option<UploadOwnership>> {
        // If this upload is owned by a user, and we are passed that user.
        if matches!(self.owner_user, Some(owner) if owner == user.id) {
            return Ok(Some(UploadOwnership::OwnedByUser));
        }

        // If this upload is owned by a team, check the user is a member of that team.
        if let Some(owner) = self.owner_team {
            if let Some(membership) =
                TeamMember::get_for_user_and_team(pool, user.id, owner).await?
            {
                return Ok(Some(UploadOwnership::OwnedByTeam(membership)));
            }
        }

        Ok(None)
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
                    if self.is_owner(pool, user).await?.is_some() {
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
                    if self.is_owner(pool, user).await?.is_some() {
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
                    if let Some(ownership) = self.is_owner(pool, user).await? {
                        return Ok(match ownership {
                            UploadOwnership::OwnedByUser => true,
                            UploadOwnership::OwnedByTeam(membership) => {
                                if permission == UploadPermission::Delete {
                                    membership.can_delete
                                } else {
                                    membership.can_edit
                                }
                            }
                        });
                    }
                }

                Ok(false)
            }
        }
    }

    pub async fn get_tags(&self, pool: &SqlitePool) -> sqlx::Result<Vec<String>> {
        sqlx::query_scalar(
            "SELECT tags.name FROM upload_tags
            LEFT JOIN tags ON tags.id = upload_tags.tag
            WHERE upload_tags.upload = $1
            ORDER BY tags.name",
        )
        .bind(self.id)
        .fetch_all(pool)
        .await
    }

    pub async fn replace_tags(&self, pool: &SqlitePool, tags: Vec<Key<Tag>>) -> sqlx::Result<()> {
        // First, delete all existing tags for this upload.
        sqlx::query("DELETE FROM upload_tags WHERE upload = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        // Then, insert the new tags.
        sqlx::query(
            "INSERT INTO upload_tags (upload, tag) \
            SELECT $1, value \
            FROM json_each($2);",
        )
        .bind(self.id)
        .bind(serde_json::to_string(&tags).expect("JSON serialization of key array"))
        .execute(pool)
        .await?;

        Ok(())
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
    pub has_password: bool,
    pub downloads: i64,
    pub limit: Option<i64>,
    pub remaining: Option<i64>,
    pub expiry_date: Option<Date>,
    pub custom_slug: Option<String>,
    pub owner_slug: String,
    pub uploaded_by_id: Option<Key<User>>,
    pub uploaded_by_name: Option<String>,
    pub uploaded_at: OffsetDateTime,
    pub tags: String,
}

impl UploadList {
    pub async fn get_for_user(
        pool: &SqlitePool,
        user: Key<User>,
        search: Option<&str>,
        order: UploadOrder,
        asc: bool,
        offset: u32,
        limit: u32,
    ) -> sqlx::Result<Vec<Self>> {
        let base_query = format!(
            "SELECT uploads.id, uploads.slug, uploads.filename, \
                uploads.size, uploads.public, uploads.downloads, \
                uploads.\"limit\", uploads.remaining, uploads.expiry_date, \
                uploads.custom_slug, \
                uploads.password IS NOT NULL AS has_password, \
                COALESCE(teams.slug, users.username) AS owner_slug, \
                uploads.uploaded_by AS uploaded_by_id, \
                uploader.name AS uploaded_by_name, \
                uploads.uploaded_at, \
                tags.tags \
                FROM uploads \
                LEFT JOIN teams ON uploads.owner_team = teams.id \
                LEFT JOIN users ON uploads.owner_user = users.id \
                LEFT JOIN users AS uploader ON uploads.uploaded_by = uploader.id \
                LEFT JOIN (\
                    SELECT \
                      upload_tags.upload, \
                      GROUP_CONCAT(tags.name, ',' ORDER BY tags.name) AS tags \
                    FROM upload_tags \
                    LEFT JOIN tags ON tags.id = upload_tags.tag \
                    GROUP BY upload_tags.upload \
                ) AS tags ON uploads.id = tags.upload \
                WHERE uploads.owner_user = $1 {} \
                ORDER BY {} {} LIMIT {} OFFSET {}",
            if search.is_some() {
                "AND (uploads.filename LIKE $2)"
            } else {
                ""
            },
            order.get_order_field(),
            if asc { "ASC" } else { "DESC" },
            limit,
            offset
        );

        if let Some(search) = search {
            let search_pattern = format!("%{search}%");
            sqlx::query_as(&base_query)
                .bind(user)
                .bind(search_pattern)
                .fetch_all(pool)
                .await
        } else {
            sqlx::query_as(&base_query)
                .bind(user)
                .fetch_all(pool)
                .await
        }
    }

    pub async fn get_for_team(
        pool: &SqlitePool,
        team: Key<Team>,
        search: Option<&str>,
        order: UploadOrder,
        asc: bool,
        offset: u32,
        limit: u32,
    ) -> sqlx::Result<Vec<Self>> {
        let base_query = format!(
            "SELECT uploads.id, uploads.slug, uploads.filename, \
                uploads.size, uploads.public, uploads.downloads, \
                uploads.\"limit\", uploads.remaining, uploads.expiry_date, \
                uploads.custom_slug, \
                uploads.password is not null as has_password, \
                COALESCE(teams.slug, users.username) AS owner_slug, \
                uploads.uploaded_by AS uploaded_by_id, \
                uploader.name AS uploaded_by_name, \
                uploads.uploaded_at, \
                tags.tags \
                FROM uploads \
                LEFT JOIN teams ON uploads.owner_team = teams.id \
                LEFT JOIN users ON uploads.owner_user = users.id \
                LEFT JOIN users AS uploader ON uploads.uploaded_by = uploader.id \
                LEFT JOIN (\
                    SELECT \
                      upload_tags.upload, \
                      GROUP_CONCAT(tags.name, ',' ORDER BY tags.name) AS tags \
                    FROM upload_tags \
                    LEFT JOIN tags ON tags.id = upload_tags.tag \
                    GROUP BY upload_tags.upload \
                ) AS tags ON uploads.id = tags.upload \
                WHERE uploads.owner_team = $1 {} \
                ORDER BY {} {} LIMIT {} OFFSET {}",
            if search.is_some() {
                "AND (uploads.filename LIKE $2)"
            } else {
                ""
            },
            order.get_order_field(),
            if asc { "ASC" } else { "DESC" },
            limit,
            offset
        );

        if let Some(search) = search {
            let search_pattern = format!("%{search}%");
            sqlx::query_as(&base_query)
                .bind(team)
                .bind(search_pattern)
                .fetch_all(pool)
                .await
        } else {
            sqlx::query_as(&base_query)
                .bind(team)
                .fetch_all(pool)
                .await
        }
    }
}
