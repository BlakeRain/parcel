use pbkdf2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Pbkdf2,
};
use rand_core::OsRng;
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

#[derive(Debug, FromRow, Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub enabled: bool,
    pub admin: bool,
    pub created_at: OffsetDateTime,
    pub created_by: Option<i32>,
}

pub async fn requires_setup(pool: &SqlitePool) -> sqlx::Result<bool> {
    let count = sqlx::query_scalar::<_, i32>("SELECT * FROM users")
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

pub fn verify_password(password: &str, plain: &str) -> bool {
    Pbkdf2
        .verify_password(
            plain.as_bytes(),
            &PasswordHash::new(&password).expect("valid password hash"),
        )
        .is_ok()
}

impl User {
    pub async fn create(&mut self, pool: &SqlitePool) -> sqlx::Result<()> {
        let result = sqlx::query_scalar::<_, i32>(
            "INSERT INTO users (username, password, enabled, admin
            created_at, created_by) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&self.username)
        .bind(&self.password)
        .bind(self.enabled)
        .bind(self.admin)
        .bind(self.created_at)
        .bind(self.created_by)
        .fetch_one(pool)
        .await?;

        self.id = result;
        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: i32) -> sqlx::Result<Option<Self>> {
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
}

#[derive(Debug, FromRow, Serialize)]
pub struct Upload {
    pub id: i32,
    pub slug: String,
    pub filename: String,
    pub public: bool,
    pub encrypted: bool,
    pub downloads: i32,
    pub remaining: Option<i32>,
    pub expiry_date: Option<OffsetDateTime>,
    pub uploaded_by: i32,
    pub uploaded_at: OffsetDateTime,
    pub remote_addr: String,
}

impl Upload {
    pub async fn create(&mut self, pool: &SqlitePool) -> sqlx::Result<()> {
        let result = sqlx::query_scalar::<_, i32>(
            "INSERT INTO uploads (slug, filename, public, encrypted,
            downloads, remaining, expiry_date,
            uploaded_by, uploaded_at, remote_addr)
            VALUES ($1, $2, $3, $4, 0, $5, $6, $7, $8, $9)
            RETURNING id",
        )
        .bind(&self.slug)
        .bind(&self.filename)
        .bind(self.public)
        .bind(self.encrypted)
        .bind(self.remaining)
        .bind(self.expiry_date)
        .bind(self.uploaded_by)
        .bind(&self.remote_addr)
        .fetch_one(pool)
        .await?;

        self.id = result;
        Ok(())
    }

    pub async fn get_by_slug(pool: &SqlitePool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM uploads WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }

    pub async fn record_download(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        if self.remaining.is_some() {
            sqlx::query(
                "UPDATE uploads SET downloads = downloads + 1,
                remaining = remaining - 1 WHERE id = ?",
            )
            .bind(self.id)
            .execute(pool)
            .await?;
        } else {
            sqlx::query("UPDATE uploads SET downloads = downloads + 1 WHERE id = ?")
                .bind(self.id)
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: i32) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM uploads WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }
}
