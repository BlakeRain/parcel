use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

use super::{team::Team, types::Key, upload::UploadOwner, user::User};

#[derive(Debug, FromRow, Serialize)]
pub struct Tag {
    pub id: Key<Tag>,
    pub name: String,
    pub user: Option<Key<User>>,
    pub team: Option<Key<Team>>,
}

impl Tag {
    pub fn new_for_user(name: String, user: Key<User>) -> Self {
        Self {
            id: Key::new(),
            name,
            user: Some(user),
            team: None,
        }
    }

    pub fn new_for_team(name: String, team: Key<Team>) -> Self {
        Self {
            id: Key::new(),
            name,
            user: None,
            team: Some(team),
        }
    }

    pub async fn create(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        let result = sqlx::query("INSERT INTO tags (id, name, user, team) VALUES ($1, $2, $3, $4)")
            .bind(self.id)
            .bind(&self.name)
            .bind(self.user)
            .bind(self.team)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    pub async fn get_for_user(pool: &SqlitePool, user_id: Key<User>) -> sqlx::Result<Vec<Tag>> {
        sqlx::query_as("SELECT * FROM tags WHERE user = $1 ORDER BY name")
            .bind(user_id)
            .fetch_all(pool)
            .await
    }

    pub async fn get_for_user_by_name(
        pool: &SqlitePool,
        user_id: Key<User>,
        name: &str,
    ) -> sqlx::Result<Option<Tag>> {
        sqlx::query_as("SELECT * FROM tags WHERE user = $1 AND name = $2")
            .bind(user_id)
            .bind(name)
            .fetch_optional(pool)
            .await
    }

    pub async fn get_or_create_for_user(
        pool: &SqlitePool,
        user_id: Key<User>,
        name: &str,
    ) -> sqlx::Result<Tag> {
        if let Some(tag) = Self::get_for_user_by_name(pool, user_id, name).await? {
            return Ok(tag);
        }

        let tag = Self::new_for_user(name.to_string(), user_id);
        tag.create(pool).await?;
        Ok(tag)
    }

    pub async fn get_for_team(pool: &SqlitePool, team_id: Key<Team>) -> sqlx::Result<Vec<Tag>> {
        sqlx::query_as("SELECT * FROM tags WHERE team = $1 ORDER BY name")
            .bind(team_id)
            .fetch_all(pool)
            .await
    }

    pub async fn get_for_team_by_name(
        pool: &SqlitePool,
        team_id: Key<Team>,
        name: &str,
    ) -> sqlx::Result<Option<Tag>> {
        sqlx::query_as("SELECT * FROM tags WHERE team = $1 AND name = $2")
            .bind(team_id)
            .bind(name)
            .fetch_optional(pool)
            .await
    }

    pub async fn get_or_create_for_team(
        pool: &SqlitePool,
        team_id: Key<Team>,
        name: &str,
    ) -> sqlx::Result<Tag> {
        if let Some(tag) = Self::get_for_team_by_name(pool, team_id, name).await? {
            return Ok(tag);
        }

        let tag = Self::new_for_team(name.to_string(), team_id);
        tag.create(pool).await?;
        Ok(tag)
    }

    pub async fn get_or_create_for_owner(
        pool: &SqlitePool,
        owner: UploadOwner,
        name: &str,
    ) -> sqlx::Result<Tag> {
        match owner {
            UploadOwner::User(user_id) => Self::get_or_create_for_user(pool, user_id, name).await,
            UploadOwner::Team(team_id) => Self::get_or_create_for_team(pool, team_id, name).await,
        }
    }
}
