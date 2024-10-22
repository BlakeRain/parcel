use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

use super::{types::Key, user::User};

#[derive(Debug, FromRow, Serialize)]
pub struct Team {
    pub id: Key<Team>,
    pub name: String,
    pub limit: Option<i64>,
    pub enabled: bool,
    pub created_at: OffsetDateTime,
    pub created_by: Key<User>,
}

impl Team {
    pub async fn create(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO teams (id, name,\"limit\", enabled, created_at, created_by) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(self.id)
            .bind(&self.name)
            .bind(self.limit)
            .bind(self.enabled)
            .bind(self.created_at)
            .bind(self.created_by)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update(
        &mut self,
        pool: &SqlitePool,
        name: &str,
        limit: Option<i64>,
        enabled: bool,
    ) -> sqlx::Result<()> {
        sqlx::query("UPDATE teams SET name = $1, \"limit\" = $2, enabled = $3 WHERE id = $4")
            .bind(name)
            .bind(limit)
            .bind(enabled)
            .bind(self.id)
            .execute(pool)
            .await?;

        self.name = name.to_string();
        self.limit = limit;
        self.enabled = enabled;

        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: Key<Team>) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM teams WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn delete(pool: &SqlitePool, id: Key<Team>) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM team_members WHERE team = $1")
            .bind(id)
            .execute(pool)
            .await?;

        sqlx::query("DELTETE FROM team WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn add_member(&self, pool: &SqlitePool, user: Key<User>) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO team_members (team, user) VALUES ($1, $2)")
            .bind(self.id)
            .bind(user)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn remove_member(&self, pool: &SqlitePool, user: Key<User>) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM team_members WHERE team = $1 AND user = $2")
            .bind(self.id)
            .bind(user)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn is_member(&self, pool: &SqlitePool, user: Key<User>) -> sqlx::Result<bool> {
        let result = sqlx::query("SELECT 1 FROM team_members WHERE team = $1 AND user = $2")
            .bind(self.id)
            .bind(user)
            .fetch_optional(pool)
            .await?;

        Ok(result.is_some())
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct TeamStats {
    pub total: i32,
}

impl TeamStats {
    pub async fn get(pool: &SqlitePool) -> sqlx::Result<TeamStats> {
        sqlx::query_as("SELECT COUNT(*) AS total FROM teams")
            .fetch_one(pool)
            .await
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct TeamList {
    pub id: Key<Team>,
    pub name: String,
    pub enabled: bool,
    pub limit: Option<i64>,
    pub upload_count: i64,
    pub upload_total: i64,
    pub member_count: i64,
    pub created_at: OffsetDateTime,
}

impl TeamList {
    pub async fn get(pool: &SqlitePool) -> sqlx::Result<Vec<TeamList>> {
        sqlx::query_as(
            "SELECT \
            teams.id, teams.name, teams.enabled, teams.\"limit\", teams.created_at, \
            COUNT(team_members.user) AS member_count, \
            COUNT(uploads.id) AS upload_count, \
            SUM(uploads.size) AS upload_total \
            FROM teams \
            LEFT JOIN team_members ON team_members.team = teams.id \
            LEFT JOIN uploads ON uploads.owner_team = teams.id \
            GROUP BY teams.id \
            ORDER BY teams.name ASC",
        )
        .fetch_all(pool)
        .await
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct TeamSelect {
    pub id: Key<Team>,
    pub name: String,
    pub enabled: bool,
}

impl TeamSelect {
    pub async fn get(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as("SELECT id, name, enabled FROM teams ORDER BY name ASC")
            .fetch_all(pool)
            .await
    }
}
