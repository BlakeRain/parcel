use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, SqlitePool};
use time::{format_description::well_known::Rfc3339, Date, OffsetDateTime};

use crate::connection::{Connection, ConnectionExt};

use super::{
    password::StoredPassword,
    team::{Team, TeamMember},
    types::Key,
    user::User,
};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
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

impl Into<libsql::Value> for UploadOrder {
    fn into(self) -> libsql::Value {
        match self {
            Self::Filename => "filename",
            Self::Size => "size",
            Self::Downloads => "downloads",
            Self::ExpiryDate => "expiry_date",
            Self::UploadedAt => "uploaded_at",
        }
        .into()
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

impl Upload {
    pub async fn create(&self, connection: &Connection) -> anyhow::Result<()> {
        connection
            .execute(
                "INSERT INTO uploads (id, slug, filename, size, public,
                downloads, \"limit\", remaining, expiry_date, password,
                custom_slug, uploaded_by, uploaded_at, remote_addr,
                owner_team, owner_user)
                VALUES (?1, ?2, ?3, ?4, ?5,
                    0, ?6, ?7, ?8, ?9,
                    ?10, ?11, ?12, ?13,
                    ?14, ?15)
                RETURNING id",
                (
                    self.id,
                    self.slug.clone(),
                    self.filename.clone(),
                    self.size,
                    self.public,
                    self.limit,
                    self.remaining,
                    match self.expiry_date {
                        Some(date) => Some(date.format(&Rfc3339).context("failed to format date")?),
                        None => None,
                    },
                    self.password.clone(),
                    self.custom_slug.clone(),
                    self.uploaded_by,
                    self.uploaded_at
                        .format(&Rfc3339)
                        .context("failed to format date")?,
                    self.remote_addr.clone(),
                    self.owner_team,
                    self.owner_user,
                ),
            )
            .await?;

        Ok(())
    }

    pub async fn save(&self, connection: &Connection) -> anyhow::Result<()> {
        let count = connection
            .execute(
                "UPDATE uploads SET
                filename = ?1,
                size = ?2,
                public = ?3,
                downloads = ?4,
                \"limit\" = ?5,
                remaining = ?6,
                expiry_date = ?7,
                password = ?8,
                custom_slug = ?9,
                owner_team = ?10,
                owner_user = ?11
                WHERE id = ?12",
                (
                    self.filename.clone(),
                    self.size,
                    self.public,
                    self.downloads,
                    self.limit,
                    self.remaining,
                    match self.expiry_date {
                        Some(date) => Some(date.format(&Rfc3339).context("failed to format date")?),
                        None => None,
                    },
                    self.password.clone(),
                    self.custom_slug.clone(),
                    self.owner_team,
                    self.owner_user,
                    self.id,
                ),
            )
            .await?;

        if count != 1 {
            anyhow::bail!("Failed to save upload");
        } else {
            Ok(())
        }
    }

    pub async fn set_public(
        &mut self,
        connection: &Connection,
        public: bool,
    ) -> anyhow::Result<()> {
        if self.public == public {
            return Ok(());
        }

        let count = connection
            .execute(
                "UPDATE uploads SET public = ?1 WHERE id = ?2",
                (public, self.id),
            )
            .await?;

        if count != 1 {
            anyhow::bail!("Failed to set public");
        }

        self.public = public;
        Ok(())
    }

    pub async fn get(connection: &Connection, id: Key<Upload>) -> anyhow::Result<Option<Self>> {
        connection
            .query_optional("SELECT * FROM uploads WHERE id = ?1", [id])
            .await
    }

    /// Fetch multiple uploads by their IDs in a single query.
    pub async fn get_many(
        connection: &Connection,
        ids: &[Key<Upload>],
    ) -> anyhow::Result<Vec<Self>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut query = vec![String::from("SELECT * FROM uploads WHERE id IN (")];

        let mut params = Vec::with_capacity(ids.len());

        for id in ids {
            query.push(format!("?{}", params.len() + 1));
            params.push(*id);
        }

        let query = query.join(", ");
        let query = format!("{query})");

        connection.query(&query, params).await
    }

    /// Delete multiple uploads by their IDs in a single query.
    /// Returns the slugs of the deleted uploads (for cache cleanup).
    pub async fn delete_many(
        connection: &Connection,
        ids: &[Key<Upload>],
    ) -> anyhow::Result<Vec<String>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut query = vec![String::from("DELETE FROM uploads WHERE id IN (")];
        let mut params = Vec::with_capacity(ids.len());

        for id in ids {
            query.push(format!("?{}", params.len() + 1));
            params.push(*id);
        }

        let query = query.join(", ");
        let query = format!("{query}) RETURNING slug");

        connection.query_scalars(&query, params).await
    }

    pub async fn get_by_slug(connection: &Connection, slug: &str) -> anyhow::Result<Option<Self>> {
        connection
            .query_optional(
                "SELECT * FROM uploads WHERE slug = ?1",
                [String::from(slug)],
            )
            .await
    }

    /// Check which slugs exist in the database from a list of candidates.
    /// Returns only the slugs that exist.
    pub async fn get_existing_slugs(
        connection: &Connection,
        slugs: &[String],
    ) -> anyhow::Result<std::collections::HashSet<String>> {
        if slugs.is_empty() {
            return Ok(std::collections::HashSet::new());
        }

        let mut query = vec![String::from("SELECT slug FROM uploads WHERE slug IN (")];
        let mut params = Vec::with_capacity(slugs.len());

        for slug in slugs {
            query.push(format!("?{}", params.len() + 1));
            params.push(slug.clone());
        }

        let query = query.join(", ");
        let query = format!("{query})");

        Ok(connection
            .query_scalars(&query, params)
            .await?
            .into_iter()
            .collect())
    }

    pub async fn get_by_custom_slug(
        connection: &Connection,
        owner: &str,
        custom_slug: &str,
    ) -> anyhow::Result<Option<Self>> {
        connection
            .query_optional(
                "SELECT uploads.* FROM uploads \
            LEFT JOIN users ON uploads.owner_user = users.id \
            LEFT JOIN teams ON uploads.owner_team = teams.id \
            WHERE (users.username = ?1 OR teams.slug = ?1) AND uploads.custom_slug = ?2",
                (String::from(owner), String::from(custom_slug)),
            )
            .await
    }

    pub async fn custom_slug_exists(
        connection: &Connection,
        owner: Key<User>,
        existing: Option<Key<Upload>>,
        custom_slug: &str,
    ) -> anyhow::Result<bool> {
        if let Some(existing) = existing {
            connection
                .query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM uploads WHERE owner_user = ?1 AND custom_slug = ?2 AND id != ?3)",
                    (owner, String::from(custom_slug), existing),
                )
                .await
        } else {
            connection
                .query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM uploads WHERE owner_user = ?1 AND custom_slug = ?2)",
                    (owner, String::from(custom_slug)),
                )
                .await
        }
    }

    pub async fn custom_team_slug_exists(
        connection: &Connection,
        owner: Key<Team>,
        existing: Option<Key<Upload>>,
        custom_slug: &str,
    ) -> anyhow::Result<bool> {
        if let Some(existing) = existing {
            connection
                .query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM uploads WHERE owner_team = ?1 AND custom_slug = ?2 AND id != ?3)",
                    (owner, String::from(custom_slug), existing),
                )
                .await
        } else {
            connection
                .query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM uploads WHERE owner_team = ?1 AND custom_slug = ?2)",
                    (owner, String::from(custom_slug)),
                )
                .await
        }
    }

    pub async fn record_download(
        &mut self,
        connection: &Connection,
        user: Option<&User>,
    ) -> anyhow::Result<()> {
        let mut query = vec![String::from("UPDATE uploads SET downloads = downloads + 1")];

        let public = match user {
            None => false,
            Some(user) => self.is_owner(connection, user).await?.is_some(),
        };

        if public && self.remaining.is_some() {
            query.push(String::from(", remaining = MAX(0, remaining - 1)"));
        }

        query.push(String::from(
            " WHERE id = ?1 RETURNING downloads, remaining",
        ));

        let (downloads, remaining) = connection.query_one(&query.join(""), [self.id]).await?;

        self.downloads = downloads;
        self.remaining = remaining;

        Ok(())
    }

    pub async fn reset_remaining(&self, connection: &Connection) -> anyhow::Result<()> {
        let count = connection
            .execute(
                "UPDATE uploads SET remaining = \"limit\" WHERE id = ?1",
                [self.id],
            )
            .await?;

        if count != 1 {
            anyhow::bail!("Failed to reset remaining downloads");
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
        connection: &Connection,
        user: &User,
    ) -> anyhow::Result<Option<UploadOwnership>> {
        // If this upload is owned by a user, and we are passed that user.
        if matches!(self.owner_user, Some(owner) if owner == user.id) {
            return Ok(Some(UploadOwnership::OwnedByUser));
        }

        // If this upload is owned by a team, check the user is a member of that team.
        if let Some(owner) = self.owner_team {
            if let Some(membership) =
                TeamMember::get_for_user_and_team(connection, user.id, owner).await?
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
                uploads.password is not null as has_password, \
                COALESCE(teams.slug, users.username) AS owner_slug, \
                uploads.uploaded_by AS uploaded_by_id, \
                uploader.name AS uploaded_by_name, \
                uploads.uploaded_at \
                FROM uploads \
                LEFT JOIN teams ON uploads.owner_team = teams.id \
                LEFT JOIN users ON uploads.owner_user = users.id \
                LEFT JOIN users AS uploader ON uploads.uploaded_by = uploader.id \
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
            sqlx::query_as(&base_query).bind(user).fetch_all(pool).await
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
                uploads.uploaded_at \
                FROM uploads \
                LEFT JOIN teams ON uploads.owner_team = teams.id \
                LEFT JOIN users ON uploads.owner_user = users.id \
                LEFT JOIN users AS uploader ON uploads.uploaded_by = uploader.id \
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
            sqlx::query_as(&base_query).bind(team).fetch_all(pool).await
        }
    }
}
