use poem::{
    error::InternalServerError,
    http::StatusCode,
    web::{Data, Json},
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{env::Env, model::user::hash_password};

async fn empty_tables(
    mut tx: sqlx::Transaction<'_, sqlx::Sqlite>,
) -> poem::Result<sqlx::Transaction<'_, sqlx::Sqlite>> {
    const TABLE_NAMES: &[&str] = &["uploads", "team_members", "teams", "users"];

    for table_name in TABLE_NAMES.iter() {
        sqlx::query(&format!("DELETE FROM {}", table_name))
            .execute(&mut *tx)
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

    Ok(tx)
}

#[poem::handler]
async fn reset_database(env: Data<&Env>) -> poem::Result<()> {
    let tx = env.pool.begin().await.map_err(|err| {
        tracing::error!(
            "Failed to begin database transaction in debug handler: {}",
            err
        );
        InternalServerError(err)
    })?;

    if let Err(err) = empty_tables(tx).await?.commit().await {
        tracing::error!(
            "Failed to commit database transaction in debug handler: {}",
            err
        );
        return Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
    };

    Ok(())
}

#[derive(Debug, Deserialize)]
struct InitialUser {
    username: String,
    name: String,
    password: String,
    admin: bool,
}

#[poem::handler]
async fn initial_users(env: Data<&Env>, Json(users): Json<Vec<InitialUser>>) -> poem::Result<()> {
    let tx = env.pool.begin().await.map_err(|err| {
        tracing::error!(
            "Failed to begin database transaction in debug handler: {}",
            err
        );
        InternalServerError(err)
    })?;

    let mut tx = empty_tables(tx).await?;

    for user in users {
        let hash = hash_password(&user.password);
        sqlx::query("INSERT INTO users (username, name, password, enabled, admin, created_at) VALUES (?, ?, ?, 1, ?, ?)")
            .bind(&user.username)
            .bind(&user.name)
            .bind(hash)
            .bind(user.admin)
            .bind(OffsetDateTime::now_utc())
            .execute(&mut *tx)
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
