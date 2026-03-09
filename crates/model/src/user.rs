use std::collections::HashSet;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::connection::{Connection, ConnectionExt};

use super::{password::StoredPassword, team::Team, types::Key, upload::UploadOrder};

#[derive(FromRow, Deserialize, Serialize)]
pub struct User {
    pub id: Key<User>,
    pub username: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub password: StoredPassword,
    #[serde(skip_serializing)]
    pub totp: Option<String>,
    pub enabled: bool,
    pub admin: bool,
    pub limit: Option<i64>,
    pub created_at: OffsetDateTime,
    pub created_by: Option<Key<User>>,
    pub last_access: Option<OffsetDateTime>,
    pub default_order: UploadOrder,
    pub default_asc: bool,
}

pub async fn requires_setup(connection: &Connection) -> anyhow::Result<bool> {
    let count = connection
        .query_scalar::<i32, _>("SELECT COUNT(*) FROM users", ())
        .await?;
    Ok(count == 0)
}

impl User {
    pub async fn create(&self, connection: &Connection) -> anyhow::Result<()> {
        let result = connection
            .execute(
                "INSERT INTO users \
            (id, username, name, password, enabled, admin, \
             \"limit\", created_at, created_by, \
             default_order, default_asc) \
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                (
                    self.id,
                    self.username.clone(),
                    self.name.clone(),
                    self.password.clone(),
                    self.enabled,
                    self.admin,
                    self.limit,
                    self.created_at
                        .format(&Rfc3339)
                        .context("failed to format date")?,
                    self.created_by,
                    self.default_order,
                    self.default_asc,
                ),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to create user");
        }

        Ok(())
    }

    pub async fn set_username(
        &mut self,
        connection: &Connection,
        username: &str,
    ) -> anyhow::Result<()> {
        let result = connection
            .execute(
                "UPDATE users SET username = ?1 WHERE id = ?2",
                (String::from(username), self.id),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to set username");
        }

        self.username = username.to_string();
        Ok(())
    }

    pub async fn set_name(&mut self, connection: &Connection, name: &str) -> anyhow::Result<()> {
        let result = connection
            .execute(
                "UPDATE users SET name = ?1 WHERE id = ?2",
                (String::from(name), self.id),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to set name");
        }

        self.name = name.to_string();
        Ok(())
    }

    pub async fn set_default_order(
        &mut self,
        connection: &Connection,
        order: UploadOrder,
        asc: bool,
    ) -> anyhow::Result<()> {
        let result = connection
            .execute(
                "UPDATE users SET default_order = ?1, default_asc = ?2 WHERE id = ?3",
                (order, asc, self.id),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to set default order");
        }

        self.default_order = order;
        self.default_asc = asc;
        Ok(())
    }

    pub async fn set_password(
        &mut self,
        connection: &Connection,
        password: &str,
    ) -> anyhow::Result<()> {
        let password = StoredPassword::new(password).context("failed to hash password")?;

        let result = connection
            .execute(
                "UPDATE users SET password = ?1 WHERE id = ?2",
                (password.clone(), self.id),
            )
            .await
            .context("failed to update user password")?;

        if result != 1 {
            anyhow::bail!("Failed to set password");
        }

        self.password = password;
        Ok(())
    }

    pub async fn set_enabled(
        &mut self,
        connection: &Connection,
        enabled: bool,
    ) -> anyhow::Result<()> {
        let result = connection
            .execute(
                "UPDATE users SET enabled = ?1 WHERE id = ?2",
                (enabled, self.id),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to set enabled");
        }

        self.enabled = enabled;
        Ok(())
    }

    pub async fn update(
        &mut self,
        connection: &Connection,
        username: &str,
        name: &str,
        admin: bool,
        enabled: bool,
        limit: Option<i64>,
    ) -> anyhow::Result<()> {
        let result = connection
            .execute(
                "UPDATE users SET \
                username = ?1, \
                name = ?2, \
                enabled = ?3, \
                admin = ?4, \
                \"limit\" = ?5 \
            WHERE id = ?6",
                (
                    String::from(username),
                    String::from(name),
                    enabled,
                    admin,
                    limit,
                    self.id,
                ),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to update user");
        }

        self.username = username.to_string();
        self.name = name.to_string();
        self.enabled = enabled;
        self.admin = admin;

        Ok(())
    }

    pub async fn record_last_access(&mut self, connection: &Connection) -> anyhow::Result<()> {
        let now = OffsetDateTime::now_utc();
        let result = connection
            .execute(
                "UPDATE users SET last_access = ?1 WHERE id = ?2",
                (
                    now.format(&Rfc3339).context("failed to format date")?,
                    self.id,
                ),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to record last access");
        }

        self.last_access = Some(now);
        Ok(())
    }

    pub async fn get(connection: &Connection, id: Key<User>) -> anyhow::Result<Option<Self>> {
        connection
            .query_optional("SELECT * FROM users WHERE id = ?1", [id])
            .await
    }

    pub async fn get_by_username(
        connection: &Connection,
        username: &str,
    ) -> anyhow::Result<Option<Self>> {
        connection
            .query_optional(
                "SELECT * FROM users WHERE username = ?1",
                [String::from(username)],
            )
            .await
    }

    pub async fn delete(&self, connection: &Connection) -> anyhow::Result<()> {
        connection
            .execute("DELETE FROM team_members WHERE user = ?1", [self.id])
            .await?;

        let result = connection
            .execute("DELETE FROM users WHERE id = ?1", [self.id])
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to delete user");
        }

        Ok(())
    }

    pub fn verify_password(&self, plain: &str) -> bool {
        self.password.verify(plain)
    }

    pub async fn set_totp_secret(
        &mut self,
        connection: &Connection,
        secret: &str,
    ) -> anyhow::Result<()> {
        let result = connection
            .execute(
                "UPDATE users SET totp = ?1 WHERE id = ?2",
                (String::from(secret), self.id),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to set TOTP secret");
        }

        self.totp = Some(secret.to_string());
        Ok(())
    }

    pub async fn remove_totp_secret(&mut self, connection: &Connection) -> anyhow::Result<()> {
        let result = connection
            .execute("UPDATE users SET totp = NULL WHERE id = ?1", [self.id])
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to remove TOTP secret");
        }

        self.totp = None;
        Ok(())
    }

    pub async fn has_teams(&self, connection: &Connection) -> anyhow::Result<bool> {
        connection
            .query_scalar(
                "SELECT EXISTS (SELECT 1 FROM team_members WHERE user = ?1)",
                [self.id],
            )
            .await
    }

    pub async fn get_teams(&self, connection: &Connection) -> anyhow::Result<HashSet<Key<Team>>> {
        Ok(connection
            .query_scalars("SELECT team FROM team_members WHERE user = ?1", [self.id])
            .await?
            .into_iter()
            .collect())
    }

    pub async fn is_member_of(
        &self,
        connection: &Connection,
        team: Key<Team>,
    ) -> anyhow::Result<bool> {
        let result = connection
            .query_optional::<(i32,), _>(
                "SELECT 1 FROM team_members WHERE user = ?1 AND team = ?2",
                (self.id, team),
            )
            .await?;

        Ok(result.is_some())
    }

    pub async fn join_team(
        &self,
        connection: &Connection,
        team: Key<Team>,
        can_edit: bool,
        can_delete: bool,
        can_config: bool,
    ) -> anyhow::Result<()> {
        let result = connection
            .execute(
                "INSERT INTO team_members \
            (team, user, can_edit, can_delete, can_config) \
            VALUES (?1, ?2, ?3, ?4, ?5)",
                (team, self.id, can_edit, can_delete, can_config),
            )
            .await?;

        if result != 1 {
            anyhow::bail!("Failed to insert team member record");
        }

        Ok(())
    }

    pub async fn leave_team(&self, connection: &Connection, team: Key<Team>) -> anyhow::Result<()> {
        connection
            .execute(
                "DELETE FROM team_members WHERE team = ?1 AND user = ?2",
                (team, self.id),
            )
            .await?;

        Ok(())
    }

    /// Remove the user from all their current teams.
    pub async fn leave_all_teams(&self, connection: &Connection) -> anyhow::Result<()> {
        connection
            .execute("DELETE FROM team_members WHERE user = ?1", [self.id])
            .await?;

        Ok(())
    }

    /// Add the user to multiple teams in a single batch INSERT.
    /// Each tuple is (team_id, can_edit, can_delete, can_config).
    pub async fn join_teams(
        &self,
        connection: &Connection,
        teams: &[(Key<Team>, bool, bool, bool)],
    ) -> anyhow::Result<()> {
        for (team, can_edit, can_delete, can_config) in teams.iter() {
            self.join_team(connection, *team, *can_edit, *can_delete, *can_config)
                .await?;
        }

        Ok(())
    }

    pub async fn username_exists(
        connection: &Connection,
        existing: Option<Key<User>>,
        slug: &str,
    ) -> anyhow::Result<bool> {
        if let Some(existing) = existing {
            connection
                .query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM users WHERE username = ?1 AND id != ?2)",
                    (String::from(slug), existing),
                )
                .await
        } else {
            connection
                .query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM users WHERE username = ?1)",
                    [String::from(slug)],
                )
                .await
        }
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct UserStats {
    pub count: i32,
    pub enabled: i32,
}

impl UserStats {
    pub async fn get(connection: &Connection) -> anyhow::Result<UserStats> {
        connection
            .query_one(
                "SELECT COUNT(*) AS count, SUM(enabled) AS enabled FROM users",
                (),
            )
            .await
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct UserList {
    pub id: Key<User>,
    pub username: String,
    pub name: String,
    pub enabled: bool,
    pub has_totp: bool,
    pub admin: bool,
    pub limit: Option<i64>,
    pub team_count: i64,
    pub upload_total: i64,
    pub created_at: OffsetDateTime,
    pub created_by_name: Option<String>,
    pub last_access: Option<OffsetDateTime>,
}

impl UserList {
    pub async fn get(connection: &Connection) -> anyhow::Result<Vec<UserList>> {
        Self::get_with_pagination(connection, 0, 50).await
    }

    pub async fn get_with_pagination(
        connection: &Connection,
        offset: u32,
        limit: u32,
    ) -> anyhow::Result<Vec<UserList>> {
        connection
            .query(
                "WITH team_counts AS (
                SELECT user, COUNT(*) AS team_count FROM team_members GROUP BY user
            ), upload_counts AS (
                SELECT uploaded_by AS user, \
                       SUM(size) AS upload_total \
                FROM uploads GROUP BY uploaded_by \
            ) \
            SELECT \
                users.id as id, \
                users.username as username, \
                users.name as name, \
                users.enabled as enabled, \
                users.totp IS NOT NULL AS has_totp, \
                users.admin as admin, \
                users.\"limit\" as \"limit\", \
                COALESCE(tc.team_count, 0) AS team_count, \
                COALESCE(uc.upload_total, 0) AS upload_total, \
                users.created_at as created_at, \
                created_by.name AS created_by_name, \
                users.last_access AS last_access \
            FROM users \
            LEFT JOIN team_counts tc ON tc.user = users.id \
            LEFT JOIN upload_counts uc ON uc.user = users.id \
            LEFT JOIN users AS created_by ON created_by.id = users.created_by \
            ORDER BY users.username \
            LIMIT ?1 OFFSET ?2",
                (limit as i64, offset as i64),
            )
            .await
    }
}
