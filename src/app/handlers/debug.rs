use poem::{
    error::InternalServerError,
    http::StatusCode,
    web::{Data, Json},
};
use serde::Deserialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use crate::{
    env::Env,
    model::{
        types::Key,
        user::{hash_password, User},
    },
};

async fn empty_tables(pool: &SqlitePool) -> poem::Result<()> {
    const TABLE_NAMES: &[&str] = &["uploads", "team_members", "teams", "users"];

    for table_name in TABLE_NAMES.iter() {
        sqlx::query(&format!("DELETE FROM {}", table_name))
            .execute(pool)
            .await
            .map_err(|err| {
                tracing::error!(
                    "Failed to delete {} in database in debug handler: {}",
                    table_name,
                    err
                );
                InternalServerError(err)
            })?;
    }

    Ok(())
}

#[poem::handler]
async fn reset_database(env: Data<&Env>) -> poem::Result<()> {
    empty_tables(&env.pool).await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct InitialUser {
    username: String,
    name: String,
    password: String,
    #[serde(default, rename = "passwordHash")]
    password_hash: Option<String>,
    admin: bool,
}

#[poem::handler]
async fn initial_users(env: Data<&Env>, Json(users): Json<Vec<InitialUser>>) -> poem::Result<()> {
    empty_tables(&env.pool).await?;

    for user in users {
        let hash = if let Some(hash) = user.password_hash {
            hash
        } else {
            hash_password(&user.password)
        };

        sqlx::query("INSERT INTO users (id, username, name, password, enabled, admin, created_at) VALUES (?, ?, ?, ?, 1, ?, ?)")
            .bind(Key::<User>::new())
            .bind(&user.username)
            .bind(&user.name)
            .bind(hash)
            .bind(user.admin)
            .bind(OffsetDateTime::now_utc())
            .execute(&env.pool)
            .await.map_err(|err| {
                tracing::error!(
                    "Failed to insert user {} in database in debug handler: {}",
                    user.username,
                    err
                );
                InternalServerError(err)
            })?;
    }

    Ok(())
}

#[cfg(debug_assertions)]
pub fn add_debug_routes(app: poem::Route) -> poem::Route {
    use poem::{get, post};

    app.at("/debug/reset-database", get(reset_database))
        .at("/debug/initial-users", post(initial_users))
}

#[cfg(not(debug_assertions))]
pub fn add_debug_routes(app: poem::Route) -> poem::Route {
    app
}
