use parcel_model::{
    password::StoredPassword,
    team::Team,
    types::Key,
    upload::{Upload, UploadList, UploadOrder, UploadPermission, UploadStats},
};
use parcel_shared::types::api::{
    ApiUpload, ApiUploadListItem, ApiUploadModifyDownloadLimit, ApiUploadModifyExpiry,
    ApiUploadModifyPassword, ApiUploadModifyRequest, ApiUploadModifySlug, ApiUploadOrder,
    ApiUploadResponse, ApiUploadSort, ApiUploadsResponse,
};
use poem::{
    http::StatusCode,
    web::{Data, Json, Path, Query},
};
use serde::Deserialize;

use crate::{app::extractors::api_key::BearerApiKey, env::Env};

#[derive(Debug, Deserialize)]
pub struct UploadsQuery {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
    #[serde(default)]
    filename: Option<String>,
    #[serde(default)]
    sort: Option<ApiUploadSort>,
    #[serde(default)]
    order: Option<ApiUploadOrder>,
}

impl UploadsQuery {
    fn get_offset(&self) -> u32 {
        self.offset.unwrap_or(0)
    }

    fn get_limit(&self) -> u32 {
        self.limit.unwrap_or(100).clamp(1, 100)
    }

    fn get_sort(&self) -> UploadOrder {
        self.sort
            .map(|sort| match sort {
                ApiUploadSort::Filename => UploadOrder::Filename,
                ApiUploadSort::Size => UploadOrder::Size,
                ApiUploadSort::Downloads => UploadOrder::Downloads,
                ApiUploadSort::ExpiryDate => UploadOrder::ExpiryDate,
                ApiUploadSort::UploadedAt => UploadOrder::UploadedAt,
            })
            .unwrap_or(UploadOrder::UploadedAt)
    }

    fn get_order(&self) -> bool {
        self.order
            .map(|order| match order {
                ApiUploadOrder::Asc => true,
                ApiUploadOrder::Desc => false,
            })
            .unwrap_or(false)
    }
}

fn api_list_item(item: UploadList) -> ApiUploadListItem {
    ApiUploadListItem {
        id: item.id.into(),
        slug: item.slug,
        filename: item.filename,
        size: item.size,
        public: item.public,
        has_password: item.has_password,
        downloads: item.downloads,
        limit: item.limit,
        remaining: item.remaining,
        expiry_date: item.expiry_date.map(|date| date.midnight().assume_utc()),
        custom_slug: item.custom_slug,
        uploaded_by_id: item.uploaded_by_id.map(|id| id.into()),
        uploaded_by_name: item.uploaded_by_name,
        uploaded_at: item.uploaded_at,
    }
}

fn api_upload(upload: Upload) -> ApiUpload {
    ApiUpload {
        id: upload.id.into(),
        slug: upload.slug,
        filename: upload.filename,
        size: upload.size,
        public: upload.public,
        has_password: upload.password.is_some(),
        downloads: upload.downloads,
        limit: upload.limit,
        remaining: upload.remaining,
        expiry_date: upload.expiry_date.map(|date| date.midnight().assume_utc()),
        custom_slug: upload.custom_slug,
        uploaded_by: upload.uploaded_by.map(|id| id.into()),
        uploaded_at: upload.uploaded_at,
    }
}

#[poem::handler]
pub async fn get_uploads(
    env: Data<&Env>,
    api_key: BearerApiKey,
    Query(query): Query<UploadsQuery>,
) -> poem::Result<Json<ApiUploadsResponse>> {
    let offset = query.get_offset();
    let limit = query.get_limit();
    let sort = query.get_sort();
    let order = query.get_order();

    let stats = UploadStats::get_for_user(&env.pool, api_key.user.id)
        .await
        .map_err(|err| {
            tracing::error!(
                api_key = ?api_key.name,
                ?err,
                "Failed to get upload stats for API key owner"
            );

            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    let uploads = UploadList::get_for_user(
        &env.pool,
        api_key.user.id,
        query.filename.as_deref(),
        sort,
        order,
        offset,
        limit,
    )
    .await
    .map_err(|err| {
        tracing::error!(
            api_key = ?api_key.name,
            ?err,
            "Failed to get uploads for API key owner"
        );

        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let uploads = uploads.into_iter().map(api_list_item).collect::<Vec<_>>();

    Ok(Json(ApiUploadsResponse {
        offset,
        total: stats.total as u32,
        total_size: stats.size,
        uploads,
    }))
}

#[poem::handler]
pub async fn get_team_uploads(
    env: Data<&Env>,
    api_key: BearerApiKey,
    Path(team_id): Path<Key<Team>>,
    Query(query): Query<UploadsQuery>,
) -> poem::Result<Json<ApiUploadsResponse>> {
    let is_member = api_key
        .user
        .is_member_of(&env.pool, team_id)
        .await
        .map_err(|err| {
            tracing::error!(
                api_key = ?api_key.name,
                team = %team_id,
                ?err,
                "Failed to check team membership"
            );

            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    if !is_member {
        tracing::error!(
            api_key = ?api_key.name,
            owner = ?api_key.user.username,
            team = %team_id,
            "API key owner is not a member of the team"
        );

        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    }

    let stats = UploadStats::get_for_team(&env.pool, team_id)
        .await
        .map_err(|err| {
            tracing::error!(
                api_key = ?api_key.name,
                team = %team_id,
                ?err,
                "Failed to get upload stats for team"
            );

            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    let offset = query.get_offset();
    let limit = query.get_limit();
    let sort = query.get_sort();
    let order = query.get_order();

    let uploads = UploadList::get_for_user(
        &env.pool,
        api_key.user.id,
        query.filename.as_deref(),
        sort,
        order,
        offset,
        limit,
    )
    .await
    .map_err(|err| {
        tracing::error!(
            api_key = ?api_key.name,
            ?err,
            "Failed to get uploads for API key owner"
        );

        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let uploads = uploads.into_iter().map(api_list_item).collect::<Vec<_>>();

    Ok(Json(ApiUploadsResponse {
        offset,
        total: stats.total as u32,
        total_size: stats.size,
        uploads,
    }))
}

#[poem::handler]
pub async fn get_upload(
    env: Data<&Env>,
    api_key: BearerApiKey,
    Path(id): Path<Key<Upload>>,
) -> poem::Result<Json<ApiUploadResponse>> {
    let Some(upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(
            api_key = ?api_key.name,
            upload = %id,
            ?err,
            "Failed to get upload by ID"
        );

        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?
    else {
        tracing::error!(
            api_key = ?api_key.name,
            upload = %id,
            "Upload not found"
        );

        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let can_access = upload
        .can_access(&env.pool, Some(&api_key.user), UploadPermission::View)
        .await
        .map_err(|err| {
            tracing::error!(
                api_key = ?api_key.name,
                upload = %id,
                ?err,
                "Failed to check upload permission"
            );

            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    if !can_access {
        tracing::error!(
            api_key = ?api_key.name,
            upload = %id,
            "API key owner does not have permission to access the upload"
        );

        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    let upload = api_upload(upload);
    Ok(Json(ApiUploadResponse { upload }))
}

#[poem::handler]
pub async fn put_upload(
    env: Data<&Env>,
    api_key: BearerApiKey,
    Path(id): Path<Key<Upload>>,
    Json(request): Json<ApiUploadModifyRequest>,
) -> poem::Result<Json<ApiUploadResponse>> {
    let Some(mut upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(
            api_key = ?api_key.name,
            upload = %id,
            ?err,
            "Failed to get upload by ID"
        );

        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?
    else {
        tracing::error!(
            api_key = ?api_key.name,
            upload = %id,
            "Upload not found"
        );

        return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
    };

    let can_access = upload
        .can_access(&env.pool, Some(&api_key.user), UploadPermission::View)
        .await
        .map_err(|err| {
            tracing::error!(
                api_key = ?api_key.name,
                upload = %id,
                ?err,
                "Failed to check upload permission"
            );

            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    if !can_access {
        tracing::error!(
            api_key = ?api_key.name,
            upload = %id,
            "API key owner does not have permission to access the upload"
        );

        return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
    }

    if let Some(ApiUploadModifySlug::Custom { ref slug }) = request.slug {
        if upload.custom_slug.as_ref() != Some(slug) {
            let exists = if let Some(owner) = upload.owner_user {
                Upload::custom_slug_exists(&env.pool, owner, Some(id), slug).await
            } else if let Some(owner) = upload.owner_team {
                Upload::custom_team_slug_exists(&env.pool, owner, Some(id), slug).await
            } else {
                tracing::error!(%id, "Upload has no owner");
                return Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
            }
            .map_err(|err| {
                tracing::error!(
                    api_key = ?api_key.name,
                    upload = %id,
                    ?err,
                    "Unable to check if custom slug exists"
                );

                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

            if exists {
                tracing::error!(
                    api_key = ?api_key.name,
                    upload = %id,
                    "Custom slug already exists"
                );

                return Err(poem::Error::from_status(StatusCode::CONFLICT));
            }
        }
    }

    if let Some(filename) = request.filename {
        upload.filename = filename;
    }

    if let Some(slug) = request.slug {
        upload.custom_slug = match slug {
            ApiUploadModifySlug::Auto => None,
            ApiUploadModifySlug::Custom { slug } => {
                let slug = slug.trim();

                if slug.is_empty() {
                    tracing::error!(
                        api_key = ?api_key.name,
                        upload = %id,
                        "Custom slug cannot be empty"
                    );

                    return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
                }

                Some(String::from(slug))
            }
        };
    }

    if let Some(public) = request.public {
        upload.public = public;
    }

    if let Some(limit) = request.limit {
        let limit = match limit {
            ApiUploadModifyDownloadLimit::Unlimited => None,
            ApiUploadModifyDownloadLimit::Limited { limit } => Some(limit),
        };

        let remaining = if upload.limit == limit {
            upload.remaining.or(limit)
        } else {
            limit
        };

        upload.limit = limit;
        upload.remaining = remaining;
    }

    if let Some(expiry) = request.expiry {
        upload.expiry_date = match expiry {
            ApiUploadModifyExpiry::Never => None,
            ApiUploadModifyExpiry::Date { date } => Some(date.date()),
        };
    }

    if let Some(password) = request.password {
        upload.password = match password {
            ApiUploadModifyPassword::None => None,
            ApiUploadModifyPassword::Set { password } => {
                if password.is_empty() {
                    tracing::error!(
                        api_key = ?api_key.name,
                        upload = %id,
                        "Password cannot be empty"
                    );

                    return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
                }

                Some(StoredPassword::new(&password).map_err(|err| {
                    tracing::error!(
                        api_key = ?api_key.name,
                        upload = %id,
                        ?err,
                        "Failed to create stored password"
                    );

                    poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
                })?)
            }
        };
    }

    upload.save(&env.pool).await.map_err(|err| {
        tracing::error!(
            api_key = ?api_key.name,
            upload = %id,
            ?err,
            "Failed to save upload"
        );

        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let upload = api_upload(upload);
    Ok(Json(ApiUploadResponse { upload }))
}

#[poem::handler]
pub async fn post_upload() -> poem::Result<()> {
    todo!()
}

#[poem::handler]
pub async fn delete_upload() -> poem::Result<()> {
    todo!()
}
