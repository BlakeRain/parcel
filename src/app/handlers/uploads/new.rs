use std::collections::HashMap;

use esbuild_bundle::javascript;
use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Html, Json, Multipart, Query, RealIp},
};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Statement};
use time::OffsetDateTime;

use crate::{
    app::{
        extractors::user::SessionUser,
        templates::{authorized_context, render_template},
    },
    env::Env,
    model::{team::Team, types::Key, upload::Upload},
};

#[derive(Debug, Deserialize)]
pub struct NewQuery {
    #[serde(default)]
    immediate: bool,
    #[serde(default)]
    team: Option<Key<Team>>,
}

#[handler]
pub async fn get_new(
    env: Data<&Env>,
    csrf_token: &CsrfToken,
    SessionUser(user): SessionUser,
    Query(NewQuery { immediate, team }): Query<NewQuery>,
) -> poem::Result<Html<String>> {
    let team = if let Some(team_id) = team {
        let Some(team) = Team::get(&env.pool, team_id).await.map_err(|err| {
            tracing::error!(%team_id, ?err, "Unable to get team by ID");
            InternalServerError(err)
        })?
        else {
            tracing::error!(%team_id, "Team not found");
            return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
        };

        let is_member = user.is_member_of(&env.pool, team.id).await.map_err(|err| {
            tracing::error!(%user.id, %team.id, "Unable to check if user is member of team");
            InternalServerError(err)
        })?;

        if !is_member {
            tracing::error!(%user.id, %team.id, "User is not a member of team");
            return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
        }

        Some(team)
    } else {
        None
    };

    render_template(
        "uploads/new.html",
        context! {
            immediate,
            team,
            csrf_token => csrf_token.0,
            upload_js => javascript!("scripts/components/upload.ts"),
            ..authorized_context(&env, &user)
        },
    )
}

#[derive(Debug, Default, Serialize)]
pub struct UploadResult {
    uploads: HashMap<String, Option<Key<Upload>>>,
}

#[handler]
pub async fn post_new(
    env: Data<&Env>,
    RealIp(ip): RealIp,
    SessionUser(user): SessionUser,
    csrf_verifier: &CsrfVerifier,
    mut form: Multipart,
) -> poem::Result<Json<UploadResult>> {
    let mut seen_csrf = false;
    let mut uploads = Vec::new();
    let mut failures = Vec::new();
    let mut team = None;

    while let Ok(Some(field)) = form.next_field().await {
        if field.name() == Some("csrf_token") {
            if !csrf_verifier.is_valid(&field.text().await?) {
                tracing::error!("CSRF token is invalid in upload form");
                return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
            }

            seen_csrf = true;
        } else if field.name() == Some("team") {
            if team.is_some() {
                tracing::error!("Multiple team fields in upload form");
                return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
            }

            let team_id = field.text().await.map_err(|err| {
                tracing::error!(?err, "Unable to read team field");
                InternalServerError(err)
            })?;

            let team_id = uuid::Uuid::parse_str(&team_id).map_err(|err| {
                tracing::error!(?err, "Unable to parse team ID");
                poem::Error::from_status(StatusCode::BAD_REQUEST)
            })?;

            team = Team::get(&env.pool, team_id.into()).await.map_err(|err| {
                tracing::error!(?err, "Unable to get team");
                InternalServerError(err)
            })?;

            match team {
                None => {
                    tracing::error!(team_id = ?team_id, "Team not found");
                    return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
                }

                Some(ref team) => {
                    let is_member = user.is_member_of(&env.pool, team.id).await.map_err(|err| {
                        tracing::error!(?err, "Unable to check if user is member of team");
                        InternalServerError(err)
                    })?;

                    if !is_member {
                        tracing::error!(team_id = ?team.id, "User is not a member of team");
                        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
                    }
                }
            }
        } else if field.name() == Some("file") {
            let slug = nanoid::nanoid!();
            let filename = field
                .file_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| "unnamed.ext".to_string());

            let mut field = field.into_async_read();
            let path = env.cache_dir.join(&slug);

            {
                let mut file = tokio::fs::File::create(&path).await.map_err(|err| {
                    tracing::error!(?err, ?path, "Unable to create file");
                    InternalServerError(err)
                })?;

                if let Err(err) = tokio::io::copy(&mut field, &mut file).await {
                    tracing::error!(?err, ?path, "Unable to copy from stream to file");
                    failures.push(filename.clone());
                    continue;
                }
            }

            let meta = tokio::fs::metadata(&path).await.map_err(|err| {
                tracing::error!(?err, ?path, "Unable to get metadata for file");
                InternalServerError(err)
            })?;

            let size = meta.len() as i64;
            tracing::info!(?slug, size, "Upload to cache complete");

            // let mut upload = Upload {
            //     id: Key::new(),
            //     slug,
            //     filename,
            //     size,
            //     public: false,
            //     downloads: 0,
            //     limit: None,
            //     remaining: None,
            //     expiry_date: None,
            //     password: None,
            //     custom_slug: None,
            //     owner_user: match team.as_ref() {
            //         Some(_) => None,
            //         None => Some(user.id),
            //     },
            //     owner_team: team.as_ref().map(|team| team.id),
            //     uploaded_by: user.id,
            //     uploaded_at: OffsetDateTime::now_utc(),
            //     remote_addr: ip.as_ref().map(ToString::to_string),
            // };

            uploads.push((slug, filename, size))
        } else {
            tracing::info!(field_name = ?field.name(), "Ignoring unrecognized field");
        }
    }

    if !seen_csrf {
        tracing::error!("CSRF token was not seen in upload form");

        for (slug, _, _) in uploads {
            let path = env.cache_dir.join(&slug);
            tracing::info!(?path, ?slug, "Deleting cached upload");
            if let Err(err) = tokio::fs::remove_file(&path).await {
                tracing::error!(?path, ?err, ?slug, "Failed to delete cached upload");
            }
        }

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let mut txn = env.pool.begin().await.map_err(|err| {
        tracing::error!(?err, "Unable to start transaction");
        InternalServerError(err)
    })?;

    let stmt = txn
        .prepare(
            "INSERT INTO uploads \
                (id, slug, filename, size, public, downloads, \
                 owner_user, owner_team, \
                 uploaded_by, uploaded_at, remote_addr) \
                VALUES (?, ?, ?, ?, 0, 0, \
                    ?, ?, \
                    ?, ?, ?)",
        )
        .await
        .map_err(|err| {
            tracing::error!(?err, "Unable to prepare statement");
            InternalServerError(err)
        })?;

    let owner_user = match team.as_ref() {
        Some(_) => None,
        None => Some(user.id),
    };

    let owner_team = team.as_ref().map(|team| team.id);
    let now = OffsetDateTime::now_utc();
    let remote_addr = ip.as_ref().map(ToString::to_string);

    let mut result = UploadResult::default();
    for (slug, filename, size) in uploads {
        let id = Key::<Upload>::new();
        stmt.query()
            .bind(id)
            .bind(&slug)
            .bind(&filename)
            .bind(size)
            .bind(owner_user)
            .bind(owner_team)
            .bind(user.id)
            .bind(now)
            .bind(&remote_addr)
            .execute(&mut *txn)
            .await
            .map_err(|err| {
                tracing::error!(?err, "Unable to insert upload");
                InternalServerError(err)
            })?;

        result.uploads.insert(filename, Some(id));
    }

    txn.commit().await.map_err(|err| {
        tracing::error!(?err, "Unable to commit transaction");
        InternalServerError(err)
    })?;

    for filename in failures {
        result.uploads.insert(filename, None);
    }

    Ok(Json(result))
}
