use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{CsrfToken, CsrfVerifier, Data, Form, Html, Path, Query},
    IntoResponse,
};
use serde::{Deserialize, Serialize};

use parcel_model::{
    team::{HomeTab, TeamMember, TeamTab},
    types::Key,
    upload::{Upload, UploadList, UploadOrder, UploadStats},
};

use crate::{
    app::{
        errors::CsrfError,
        extractors::user::SessionUser,
        handlers::utils::delete_upload_cache_by_slug,
        templates::{authorized_context, render_template},
    },
    env::Env,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub search: String,
    pub order: Option<UploadOrder>,
    pub asc: Option<bool>,
}

impl ListQuery {
    pub fn get_search(&self) -> Option<&str> {
        let search = self.search.trim();
        if search.is_empty() {
            None
        } else {
            Some(search)
        }
    }
}

#[handler]
pub async fn get_list(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_token: &CsrfToken,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let has_teams = user.has_teams(&env.pool).await.map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to check if user has teams");
        InternalServerError(err)
    })?;

    let home = HomeTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get home tab for user");
            poem::error::InternalServerError(err)
        })?;

    let tabs = TeamTab::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(%user.id, ?err, "Unable to get team tabs for user");
            poem::error::InternalServerError(err)
        })?;

    let stats = UploadStats::get_for_user(&env.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!(?err, %user.id, "Failed to get upload stats for user");
            InternalServerError(err)
        })?;

    let uploads = UploadList::get_for_user(
        &env.pool,
        user.id,
        query.get_search(),
        query.order.unwrap_or(user.default_order),
        query.asc.unwrap_or(user.default_asc),
        0,
        50,
    )
    .await
    .map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to get uploads for user");
        InternalServerError(err)
    })?;

    render_template(
        "uploads/list.html",
        context! {
            home,
            tabs,
            stats,
            uploads,
            has_teams,
            query,
            csrf_token => csrf_token.0,
            page => 0,
            limit => user.limit,
            ..authorized_context(&env, &user)
        },
    )
    .await
}

#[handler]
pub async fn post_delete(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    csrf_verifier: &CsrfVerifier,
    Form(form): Form<Vec<(String, String)>>,
) -> poem::Result<impl IntoResponse> {
    let csrf_token = form
        .iter()
        .find(|(name, _)| name == "csrf_token")
        .map(|(_, token)| token)
        .ok_or_else(|| {
            tracing::error!("CSRF token not found in form data");
            poem::Error::from_status(StatusCode::BAD_REQUEST)
        })?;

    if !csrf_verifier.is_valid(csrf_token) {
        tracing::error!("Invalid CSRF token in form data");
        return Err(CsrfError.into());
    }

    let ids: Vec<Key<Upload>> = form
        .into_iter()
        .filter(|(name, _)| name == "selected")
        .map(|(_, id)| id.parse::<Key<Upload>>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            tracing::error!("Invalid upload ID in form data");
            poem::Error::from_status(StatusCode::BAD_REQUEST)
        })?;

    if ids.is_empty() {
        return Ok(Html("").with_header("HX-Refresh", "true"));
    }

    // Batch fetch all uploads in a single query
    let uploads = Upload::get_many(&env.pool, &ids).await.map_err(|err| {
        tracing::error!(?err, "Unable to fetch uploads for bulk delete");
        InternalServerError(err)
    })?;

    // Verify all requested IDs were found
    if uploads.len() != ids.len() {
        tracing::error!(
            requested = ids.len(),
            found = uploads.len(),
            "Some uploads not found for bulk delete"
        );
        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    }

    // Pre-fetch user's team memberships once for permission checking
    let team_memberships = if user.admin {
        Vec::new() // Admin can delete anything, no need to fetch
    } else {
        TeamMember::get_for_user(&env.pool, user.id)
            .await
            .map_err(|err| {
                tracing::error!(?err, %user.id, "Unable to fetch team memberships");
                InternalServerError(err)
            })?
    };

    // Check permissions for each upload in-memory
    let mut ids_to_delete = Vec::with_capacity(uploads.len());
    for upload in &uploads {
        let can_delete = if user.admin {
            true
        } else if upload.owner_user == Some(user.id) {
            // User owns the upload directly
            true
        } else if let Some(team_id) = upload.owner_team {
            // Check if user is a member of the team with delete permission
            team_memberships
                .iter()
                .any(|m| m.team == team_id && m.can_delete)
        } else {
            false
        };

        if !can_delete {
            tracing::error!(
                upload = %upload.id,
                user = %user.id,
                "User tried to delete upload without permission"
            );
            return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
        }

        ids_to_delete.push(upload.id);
    }

    // Batch delete all uploads in a single query
    let deleted_slugs = Upload::delete_many(&env.pool, &ids_to_delete)
        .await
        .map_err(|err| {
            tracing::error!(?err, "Unable to delete uploads");
            InternalServerError(err)
        })?;

    // Delete cache files for each deleted upload
    for slug in deleted_slugs {
        delete_upload_cache_by_slug(&env, &slug).await;
    }

    Ok(Html("").with_header("HX-Refresh", "true"))
}

#[handler]
pub async fn get_page(
    env: Data<&Env>,
    SessionUser(user): SessionUser,
    Path(page): Path<u32>,
    Query(query): Query<ListQuery>,
) -> poem::Result<Html<String>> {
    let has_teams = user.has_teams(&env.pool).await.map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to check if user has teams");
        InternalServerError(err)
    })?;

    let uploads = UploadList::get_for_user(
        &env.pool,
        user.id,
        query.get_search(),
        query.order.unwrap_or(user.default_order),
        query.asc.unwrap_or(user.default_asc),
        50 * page,
        50,
    )
    .await
    .map_err(|err| {
        tracing::error!(%user.id, ?err, "Unable to get uploads for user");
        InternalServerError(err)
    })?;

    render_template(
        "uploads/page.html",
        context! {
            page,
            uploads,
            has_teams,
            query,
            ..authorized_context(&env, &user)
        },
    )
    .await
}
