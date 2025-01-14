use std::collections::HashSet;

use anyhow::Context;
use serde::Serialize;
use sqlx::{FromRow, QueryBuilder, SqlitePool};
use time::OffsetDateTime;

use super::{password::StoredPassword, team::Team, types::Key};

#[derive(FromRow, Serialize)]
pub struct User {
    pub id: Key<User>,
    pub username: String,
    pub name: String,
    #[serde(skip)]
    pub password: StoredPassword,
    #[serde(skip)]
    pub totp: Option<String>,
    pub enabled: bool,
    pub admin: bool,
    pub limit: Option<i64>,
    pub created_at: OffsetDateTime,
    pub created_by: Option<Key<User>>,
}

pub async fn requires_setup(pool: &SqlitePool) -> sqlx::Result<bool> {
    let count = sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    Ok(count == 0)
}

impl User {
    pub async fn create(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO users (id, username, name, password, enabled, admin,
            \"limit\", created_at, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id",
        )
        .bind(self.id)
        .bind(&self.username)
        .bind(&self.name)
        .bind(&self.password)
        .bind(self.enabled)
        .bind(self.admin)
        .bind(self.limit)
        .bind(self.created_at)
        .bind(self.created_by)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_username(&mut self, pool: &SqlitePool, username: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE users SET username = $1 WHERE id = $2")
            .bind(username)
            .bind(self.id)
            .execute(pool)
            .await?;

        self.username = username.to_string();
        Ok(())
    }

    pub async fn set_name(&mut self, pool: &SqlitePool, name: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE users SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(self.id)
            .execute(pool)
            .await?;

        self.name = name.to_string();
        Ok(())
    }

    pub async fn set_password(&mut self, pool: &SqlitePool, password: &str) -> anyhow::Result<()> {
        let password = StoredPassword::new(password).context("failed to hash password")?;

        sqlx::query("UPDATE users SET password = $1 WHERE id = $2")
            .bind(&password)
            .bind(self.id)
            .execute(pool)
            .await
            .context("failed to update user password")?;

        self.password = password;
        Ok(())
    }

    pub async fn set_enabled(&mut self, pool: &SqlitePool, enabled: bool) -> sqlx::Result<()> {
        sqlx::query("UPDATE users SET enabled = $1 WHERE id = $2")
            .bind(enabled)
            .bind(self.id)
            .execute(pool)
            .await?;

        self.enabled = enabled;
        Ok(())
    }

    pub async fn update(
        &mut self,
        pool: &SqlitePool,
        username: &str,
        name: &str,
        admin: bool,
        enabled: bool,
        limit: Option<i64>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE users SET
                    username = $1, name = $2, enabled = $3, admin = $4, \"limit\" = $5
                    WHERE id = $6",
        )
        .bind(username)
        .bind(name)
        .bind(enabled)
        .bind(admin)
        .bind(limit)
        .bind(self.id)
        .execute(pool)
        .await?;

        self.username = username.to_string();
        self.name = name.to_string();
        self.enabled = enabled;
        self.admin = admin;

        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: Key<User>) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn get_by_username(pool: &SqlitePool, username: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await
    }

    pub async fn delete(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM team_members WHERE user = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub fn verify_password(&self, plain: &str) -> bool {
        self.password.verify(plain)
    }

    pub async fn set_totp_secret(&mut self, pool: &SqlitePool, secret: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE users SET totp = $1 WHERE id = $2")
            .bind(secret)
            .bind(self.id)
            .execute(pool)
            .await?;

        self.totp = Some(secret.to_string());
        Ok(())
    }

    pub async fn remove_totp_secret(&mut self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query("UPDATE users SET totp = NULL WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        self.totp = None;
        Ok(())
    }

    pub async fn has_teams(&self, pool: &SqlitePool) -> sqlx::Result<bool> {
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM team_members WHERE user = $1)")
            .bind(self.id)
            .fetch_one(pool)
            .await
    }

    pub async fn get_teams(&self, pool: &SqlitePool) -> sqlx::Result<HashSet<Key<Team>>> {
        Ok(
            sqlx::query_scalar("SELECT team FROM team_members WHERE user = $1")
                .bind(self.id)
                .fetch_all(pool)
                .await?
                .into_iter()
                .collect(),
        )
    }

    pub async fn is_member_of(&self, pool: &SqlitePool, team: Key<Team>) -> sqlx::Result<bool> {
        let result = sqlx::query("SELECT 1 FROM team_members WHERE user = $1 AND team = $2")
            .bind(self.id)
            .bind(team)
            .fetch_optional(pool)
            .await?;

        Ok(result.is_some())
    }

    pub async fn join_team(
        &self,
        pool: &SqlitePool,
        team: Key<Team>,
        can_edit: bool,
        can_delete: bool,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO team_members (team, user, can_edit, can_delete) VALUES ($1, $2, $3, $4)",
        )
        .bind(team)
        .bind(self.id)
        .bind(can_edit)
        .bind(can_delete)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn leave_team(&self, pool: &SqlitePool, team: Key<Team>) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM team_members WHERE team = $1 AND user = $2")
            .bind(team)
            .bind(self.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn username_exists(
        pool: &SqlitePool,
        existing: Option<Key<User>>,
        slug: &str,
    ) -> sqlx::Result<bool> {
        let mut query = QueryBuilder::new("SELECT EXISTS (SELECT 1 FROM users WHERE username = ");
        query.push_bind(slug);

        if let Some(existing) = existing {
            query.push(" AND id != ");
            query.push_bind(existing);
        }

        query.push(")");

        query.build_query_scalar().fetch_one(pool).await
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct UserStats {
    pub count: i32,
    pub enabled: i32,
}

impl UserStats {
    pub async fn get(pool: &SqlitePool) -> sqlx::Result<UserStats> {
        sqlx::query_as("SELECT COUNT(*) AS count, SUM(enabled) AS enabled FROM users")
            .fetch_one(pool)
            .await
    }
}

#[derive(Debug, FromRow, Serialize)]
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
}

impl UserList {
    pub async fn get(pool: &SqlitePool) -> sqlx::Result<Vec<UserList>> {
        sqlx::query_as(
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
                created_by.name AS created_by_name \
            FROM users \
            LEFT JOIN team_counts tc ON tc.user = users.id \
            LEFT JOIN upload_counts uc ON uc.user = users.id \
            LEFT JOIN users AS created_by ON created_by.id = users.created_by \
            ORDER BY users.username",
        )
        .fetch_all(pool)
        .await
    }
}
