use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiMeResponse {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    #[serde(rename = "lastAccess", with = "time::serde::rfc3339::option")]
    pub last_access: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiTeamsResponse {
    pub teams: Vec<ApiTeamInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiTeamResponse {
    #[serde(flatten)]
    pub team: ApiTeamInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiTeamInfo {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUploadsResponse {
    pub offset: u32,
    pub total: u32,
    #[serde(rename = "totalSize")]
    pub total_size: i64,
    pub uploads: Vec<ApiUploadListItem>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiUploadSort {
    #[serde(rename = "filename")]
    Filename,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "downloads")]
    Downloads,
    #[serde(rename = "expiryDate")]
    ExpiryDate,
    #[serde(rename = "uploadedAt")]
    UploadedAt,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiUploadOrder {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUploadListItem {
    pub id: Uuid,
    pub slug: String,
    pub filename: String,
    pub size: i64,
    pub public: bool,
    #[serde(rename = "hasPassword")]
    pub has_password: bool,
    pub downloads: i64,
    pub limit: Option<i64>,
    pub remaining: Option<i64>,
    #[serde(rename = "expiryDate", with = "time::serde::rfc3339::option")]
    pub expiry_date: Option<OffsetDateTime>,
    #[serde(rename = "customSlug")]
    pub custom_slug: Option<String>,
    #[serde(rename = "uploadedById")]
    pub uploaded_by_id: Option<Uuid>,
    #[serde(rename = "uploadedByName")]
    pub uploaded_by_name: Option<String>,
    #[serde(rename = "uploadedAt", with = "time::serde::rfc3339")]
    pub uploaded_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUploadResponse {
    pub upload: ApiUpload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUpload {
    pub id: Uuid,
    pub slug: String,
    pub filename: String,
    pub size: i64,
    pub public: bool,
    pub has_password: bool,
    pub downloads: i64,
    pub limit: Option<i64>,
    pub remaining: Option<i64>,
    #[serde(rename = "expiryDate", with = "time::serde::rfc3339::option")]
    pub expiry_date: Option<OffsetDateTime>,
    #[serde(rename = "customSlug")]
    pub custom_slug: Option<String>,
    pub uploaded_by: Option<Uuid>,
    pub uploaded_at: OffsetDateTime,
}
