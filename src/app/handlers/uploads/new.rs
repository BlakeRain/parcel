use esbuild_bundle::javascript;
use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Html, Json, Multipart, Query, RealIp},
};
use serde::Deserialize;
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
    .await
}

#[handler]
pub async fn post_new(
    env: Data<&Env>,
    RealIp(ip): RealIp,
    SessionUser(user): SessionUser,
    csrf_verifier: &CsrfVerifier,
    mut form: Multipart,
) -> poem::Result<Json<()>> {
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
            let filename = field
                .file_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| "unnamed.ext".to_string());

            let (slug, path) = {
                loop {
                    let slug = nanoid::nanoid!();
                    let path = env.cache_dir.join(&slug);

                    if !path.exists() {
                        break (slug, path);
                    }

                    tracing::info!(?slug, ?path, "Slug already exists, generating a new one");
                }
            };

            let mut field = field.into_async_read();

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

            uploads.push(serde_json::json!({
                "id": Key::<Upload>::new(),
                "slug": slug,
                "filename": filename,
                "size": size,
            }));
        } else {
            tracing::info!(field_name = ?field.name(), "Ignoring unrecognized field");
        }
    }

    if !seen_csrf {
        tracing::error!("CSRF token was not seen in upload form");

        for upload in uploads {
            let slug = upload
                .get("slug")
                .expect("slug field missing")
                .as_str()
                .expect("slug field is not a string");

            let path = env.cache_dir.join(slug);
            tracing::info!(?path, ?slug, "Deleting cached upload");
            if let Err(err) = tokio::fs::remove_file(&path).await {
                tracing::error!(?path, ?err, ?slug, "Failed to delete cached upload");
            }
        }

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    let owner_user = match team.as_ref() {
        Some(_) => None,
        None => Some(user.id),
    };

    let owner_team = team.as_ref().map(|team| team.id);
    let remote_addr = ip.as_ref().map(ToString::to_string);

    sqlx::query(
        "\
        WITH data AS ( \
            SELECT value ->> 'id' AS id, \
                   value ->> 'slug' AS slug, \
                   value ->> 'filename' AS filename, \
                   (value ->> 'size') AS size \
            FROM json_each($1)) \
        INSERT INTO uploads \
        (id, slug, filename, size, public, downloads, \
         owner_user, owner_team, \
         uploaded_at, uploaded_by, remote_addr) \
        SELECT data.id, data.slug, data.filename, data.size, 0, 0, \
               $2, $3, \
               $4, $5, $6 \
        FROM data",
    )
    .bind(serde_json::to_string(&uploads).expect("JSON serialization failed"))
    .bind(owner_user)
    .bind(owner_team)
    .bind(OffsetDateTime::now_utc())
    .bind(user.id)
    .bind(remote_addr)
    .execute(&env.pool)
    .await
    .map_err(|err| {
        tracing::error!(?err, "Unable to insert uploads");
        InternalServerError(err)
    })?;

    Ok(Json(()))
}
