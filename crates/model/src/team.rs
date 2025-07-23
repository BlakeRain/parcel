use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, SqlitePool};
use time::OffsetDateTime;

use super::{types::Key, user::User};

#[derive(Debug, FromRow, Serialize)]
pub struct Team {
    pub id: Key<Team>,
    pub name: String,
    pub slug: String,
    pub limit: Option<i64>,
    pub enabled: bool,
    pub created_at: OffsetDateTime,
    pub created_by: Key<User>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum TeamPermission {
    Edit,
    Delete,
}

impl Team {
    pub async fn create(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO teams (id, name, slug, \"limit\", enabled, created_at, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(self.id)
            .bind(&self.name)
            .bind(&self.slug)
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
        slug: &str,
        limit: Option<i64>,
        enabled: bool,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE teams SET name = $1, slug = $2, \"limit\" = $3, enabled = $4 WHERE id = $5",
        )
        .bind(name)
        .bind(slug)
        .bind(limit)
        .bind(enabled)
        .bind(self.id)
        .execute(pool)
        .await?;

        self.name = name.to_string();
        self.slug = slug.to_string();
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

    pub async fn get_by_slug(pool: &SqlitePool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM teams WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }

    pub async fn get_list(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as("SELECT * FROM teams ORDER BY name ASC")
            .fetch_all(pool)
            .await
    }

    pub async fn get_for_user(pool: &SqlitePool, user: Key<User>) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as("SELECT teams.* FROM teams LEFT JOIN team_members ON team_members.team = teams.id WHERE team_members.user = $1")
            .bind(user)
            .fetch_all(pool)
            .await
    }

    pub async fn delete(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM team_members WHERE team = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn slug_exists(
        pool: &SqlitePool,
        existing: Option<Key<Team>>,
        slug: &str,
    ) -> sqlx::Result<bool> {
        let mut query = QueryBuilder::new("SELECT EXISTS (SELECT 1 FROM teams WHERE slug = ");
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
pub struct TeamMember {
    pub team: Key<Team>,
    pub user: Key<User>,
    pub can_edit: bool,
    pub can_delete: bool,
}

impl TeamMember {
    pub async fn get_for_user(pool: &SqlitePool, user: Key<User>) -> sqlx::Result<Vec<TeamMember>> {
        sqlx::query_as("SELECT * FROM team_members WHERE user = $1")
            .bind(user)
            .fetch_all(pool)
            .await
    }

    pub async fn get_for_user_and_team(
        pool: &SqlitePool,
        user: Key<User>,
        team: Key<Team>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM team_members WHERE user = $1 AND team = $2")
            .bind(user)
            .bind(team)
            .fetch_optional(pool)
            .await
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
pub struct TeamTab {
    pub id: Key<Team>,
    pub name: String,
    pub slug: String,
    pub count: i64,
}

impl TeamTab {
    pub async fn get_for_user(pool: &SqlitePool, user: Key<User>) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT teams.id, teams.name, teams.slug, COUNT(uploads.id) AS count \
            FROM teams \
            LEFT JOIN uploads ON uploads.owner_team = teams.id \
            LEFT JOIN team_members ON team_members.team = teams.id \
            WHERE team_members.user = $1 \
            GROUP BY teams.id",
        )
        .bind(user)
        .fetch_all(pool)
        .await
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct HomeTab {
    pub count: i64,
}

impl HomeTab {
    pub async fn get_for_user(pool: &SqlitePool, user: Key<User>) -> sqlx::Result<Self> {
        sqlx::query_as(
            "SELECT COUNT(uploads.id) AS count \
            FROM uploads \
            WHERE uploads.owner_user = $1",
        )
        .bind(user)
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
            (SELECT COUNT(*) FROM team_members WHERE team_members.team = teams.id) AS member_count, \
            COUNT(uploads.id) AS upload_count, \
            SUM(uploads.size) AS upload_total \
            FROM teams \
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
    pub value: Key<Team>,
    pub label: String,
    pub enabled: bool,
}

impl TeamSelect {
    pub async fn get(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as("SELECT id as value, name as label, enabled FROM teams ORDER BY name ASC")
            .fetch_all(pool)
            .await
    }
}
