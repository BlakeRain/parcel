use std::collections::HashSet;

use pbkdf2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Pbkdf2,
};
use rand_core::OsRng;
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

use super::{team::Team, types::Key};

#[derive(Debug, FromRow, Serialize)]
pub struct User {
    pub id: Key<User>,
    pub username: String,
    pub name: String,
    #[serde(skip)]
    pub password: String,
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

pub fn hash_password(plain: &str) -> String {
    Pbkdf2
        .hash_password(plain.as_bytes(), &SaltString::generate(&mut OsRng))
        .expect("password hash")
        .to_string()
}

pub fn verify_password(hash: &str, plain: &str) -> bool {
    Pbkdf2
        .verify_password(
            plain.as_bytes(),
            &PasswordHash::new(hash).expect("valid password hash"),
        )
        .is_ok()
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

    pub async fn set_password(&mut self, pool: &SqlitePool, password: &str) -> sqlx::Result<()> {
        let password = hash_password(password);

        sqlx::query("UPDATE users SET password = $1 WHERE id = $2")
            .bind(&password)
            .bind(self.id)
            .execute(pool)
            .await?;

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

    pub async fn get_list(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as("SELECT * FROM users ORDER BY username")
            .fetch_all(pool)
            .await
    }

    pub async fn delete(pool: &SqlitePool, id: Key<User>) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub fn verify_password(&self, plain: &str) -> bool {
        verify_password(&self.password, plain)
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

    pub async fn join_team(&self, pool: &SqlitePool, team: Key<Team>) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO team_members (team, user) VALUES ($1, $2)")
            .bind(team)
            .bind(self.id)
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
}

#[derive(Debug, FromRow, Serialize)]
pub struct UserStats {
    pub total: i32,
    pub enabled: i32,
}

impl UserStats {
    pub async fn get(pool: &SqlitePool) -> sqlx::Result<UserStats> {
        sqlx::query_as("SELECT COUNT(*) AS total, SUM(enabled) AS enabled FROM users")
            .fetch_one(pool)
            .await
    }
}
