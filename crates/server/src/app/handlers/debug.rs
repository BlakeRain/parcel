use anyhow::Context;
use base64::Engine;
use poem::{
    error::InternalServerError,
    http::StatusCode,
    web::{Data, Json},
};
use serde::Deserialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use parcel_model::{password::StoredPassword, team::Team, types::Key, upload::Upload, user::User};

use crate::env::Env;

async fn empty_tables(pool: &SqlitePool) -> poem::Result<()> {
    const TABLE_NAMES: &[&str] = &["uploads", "team_members", "teams", "users"];

    for table_name in TABLE_NAMES.iter() {
        sqlx::query(&format!("DELETE FROM {table_name}"))
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
            StoredPassword::try_from(hash.as_str())
                .context("failed to parse password hash for user")?
        } else {
            StoredPassword::new(&user.password).context("failed to hash password for user")?
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

#[derive(Debug, Deserialize)]
pub struct InitialTeam {
    name: String,
    slug: String,
    members: Vec<InitialTeamMember>,
}

#[derive(Debug, Deserialize)]
struct InitialTeamMember {
    name: String,
    edit: bool,
    delete: bool,
    config: bool,
}

#[poem::handler]
async fn post_initial_teams(
    env: Data<&Env>,
    Json(teams): Json<Vec<InitialTeam>>,
) -> poem::Result<()> {
    for InitialTeam {
        name,
        slug,
        members,
    } in teams
    {
        let team = Team {
            id: Key::new(),
            name,
            slug,
            limit: None,
            enabled: true,
            created_at: OffsetDateTime::now_utc(),
            created_by: None,
        };

        team.create(&env.pool).await.map_err(|err| {
            tracing::error!(?err, "Failed to insert team in database");
            InternalServerError(err)
        })?;

        for member in members {
            let Some(user) = User::get_by_username(&env.pool, &member.name)
                .await
                .map_err(|err| {
                    tracing::error!("Failed to get user by username: {}", err);
                    InternalServerError(err)
                })?
            else {
                tracing::error!("User not found: {}", member.name);
                return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
            };

            user.join_team(
                &env.pool,
                team.id,
                member.edit,
                member.delete,
                member.config,
            )
            .await
            .map_err(|err| {
                tracing::error!(?err, "Failed to add user to team in database");
                InternalServerError(err)
            })?;
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct DirectUpload {
    owner: String,
    filename: String,
    content: String,
}

#[poem::handler]
async fn post_uploads(
    env: Data<&Env>,
    Json(uploads): Json<Vec<DirectUpload>>,
) -> poem::Result<Json<Vec<Upload>>> {
    let mut result = Vec::new();

    for DirectUpload {
        owner,
        filename,
        content,
    } in uploads
    {
        let owner_user = User::get_by_username(&env.pool, &owner)
            .await
            .map_err(|err| {
                tracing::error!("Failed to get user by username: {}", err);
                InternalServerError(err)
            })?;

        let owner_team = Team::get_by_slug(&env.pool, &owner).await.map_err(|err| {
            tracing::error!("Failed to get team by slug: {}", err);
            InternalServerError(err)
        })?;

        if owner_user.is_none() && owner_team.is_none() {
            tracing::error!("Owner not found: {}", owner);
            return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
        }

        if owner_user.is_some() && owner_team.is_some() {
            tracing::error!("Both user and team found for owner: {}", owner);
            return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
        }

        let content = base64::engine::general_purpose::STANDARD
            .decode(content.as_bytes())
            .map_err(|err| {
                tracing::error!(?err, "Failed to decode base64 content");
                poem::Error::from_status(StatusCode::BAD_REQUEST)
            })?;

        let slug = nanoid::nanoid!();
        let path = env.cache_dir.join(&slug);
        tokio::fs::write(&path, &content).await.map_err(|err| {
            tracing::error!(?path, ?err, "Failed to write file");
            InternalServerError(err)
        })?;

        let upload = Upload {
            id: Key::new(),
            slug,
            filename,
            size: content.len() as i64,
            public: false,
            downloads: 0,
            limit: None,
            remaining: None,
            expiry_date: None,
            password: None,
            custom_slug: None,
            owner_team: owner_team.map(|t| t.id),
            owner_user: owner_user.map(|u| u.id),
            uploaded_by: None,
            uploaded_at: OffsetDateTime::now_utc(),
            remote_addr: None,
            mime_type: None,
            has_preview: false,
            preview_error: None,
        };

        upload.create(&env.pool).await.map_err(|err| {
            tracing::error!(?err, "Failed to insert upload in database");
            InternalServerError(err)
        })?;

        result.push(upload);
    }

    Ok(Json(result))
}

pub fn add_debug_routes(app: poem::Route) -> poem::Route {
    use poem::{get, post};

    app.at("/debug/reset-database", get(reset_database))
        .at("/debug/initial-users", post(initial_users))
        .at("/debug/initial-teams", post(post_initial_teams))
        .at("/debug/uploads", post(post_uploads))
}
