use parcel_shared::types::api::{
    ApiMeResponse, ApiTeamResponse, ApiTeamsResponse, ApiUploadOrder, ApiUploadResponse,
    ApiUploadSort, ApiUploadsResponse,
};
use reqwest::{
    header::{AUTHORIZATION, USER_AGENT},
    Method,
};
use typed_builder::TypedBuilder;
use uuid::Uuid;

static PARCEL_USER_AGENT: &str = concat!("parcel/", env!("CARGO_PKG_VERSION"));

pub struct Client {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl Client {
    pub fn build() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn new(base_url: String, api_key: String) -> Self {
        let client = reqwest::Client::new();
        Client {
            client,
            base_url,
            api_key,
        }
    }

    pub fn new_with_client(base_url: String, api_key: String, client: reqwest::Client) -> Self {
        Client {
            client,
            base_url,
            api_key,
        }
    }

    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }

    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }

    pub fn get_client(&self) -> &reqwest::Client {
        &self.client
    }

    pub fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        self.client
            .request(method, &url)
            .header(USER_AGENT, PARCEL_USER_AGENT)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
    }

    pub async fn get_me(&self) -> Result<ApiMeResponse, ClientError> {
        let response = self.request(Method::GET, "/api/1.0/me").send().await?;
        let response = response.json().await?;
        Ok(response)
    }

    pub async fn get_teams(&self) -> Result<ApiTeamsResponse, ClientError> {
        let response = self.request(Method::GET, "/api/1.0/teams").send().await?;
        let response = response.json().await?;
        Ok(response)
    }

    pub async fn get_team(&self, team_id: Uuid) -> Result<ApiTeamResponse, ClientError> {
        let response = self
            .request(Method::GET, &format!("/api/1.0/teams/{team_id}"))
            .send()
            .await?;
        let response = response.json().await?;
        Ok(response)
    }

    pub async fn get_uploads(
        &self,
        query: UploadsListQuery,
    ) -> Result<ApiUploadsResponse, ClientError> {
        let mut builder = self.request(Method::GET, "/api/1.0/uploads");
        let params = query.into_params();

        if !params.is_empty() {
            builder = builder.query(&params);
        }

        let response = builder.send().await?;
        let response = response.json().await?;

        Ok(response)
    }

    pub async fn get_team_uploads(
        &self,
        team_id: Uuid,
        query: UploadsListQuery,
    ) -> Result<ApiUploadsResponse, ClientError> {
        let mut builder = self.request(Method::GET, &format!("/api/1.0/teams/{team_id}/uploads"));
        let params = query.into_params();

        if !params.is_empty() {
            builder = builder.query(&params);
        }

        let response = builder.send().await?;
        let response = response.json().await?;

        Ok(response)
    }

    pub async fn get_upload(&self, upload_id: Uuid) -> Result<ApiUploadResponse, ClientError> {
        let response = self
            .request(Method::GET, &format!("/api/1.0/uploads/{upload_id}"))
            .send()
            .await?;
        let response = response.json().await?;
        Ok(response)
    }
}

#[derive(Debug, Default, Clone, TypedBuilder)]
pub struct UploadsListQuery {
    filename: Option<String>,
    sort: Option<ApiUploadSort>,
    order: Option<ApiUploadOrder>,
    limit: Option<u32>,
    offset: Option<u32>,
}

impl UploadsListQuery {
    fn into_params(self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();

        if let Some(filename) = self.filename {
            params.push(("filename", filename));
        }
        if let Some(sort) = self.sort {
            params.push(("sort", sort.to_string()));
        }
        if let Some(order) = self.order {
            params.push(("order", order.to_string()));
        }
        if let Some(limit) = self.limit {
            params.push(("limit", limit.to_string()));
        }
        if let Some(offset) = self.offset {
            params.push(("offset", offset.to_string()));
        }

        params
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("JSON deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("API error: {0}")]
    ApiError(String),
}

#[derive(Debug, Default)]
pub struct ClientBuilder {
    client: Option<reqwest::Client>,
    base_url: Option<String>,
    api_key: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientBuilderError {
    #[error("Missing API key")]
    MissingApiKey,
    #[error("Missing base URL")]
    MissingBaseUrl,
}

impl ClientBuilder {
    pub fn new() -> Self {
        ClientBuilder {
            client: None,
            base_url: None,
            api_key: None,
        }
    }

    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn build(self) -> Result<Client, ClientBuilderError> {
        let client = self.client.unwrap_or_default();
        let base_url = self.base_url.ok_or(ClientBuilderError::MissingBaseUrl)?;
        let api_key = self.api_key.ok_or(ClientBuilderError::MissingApiKey)?;
        Ok(Client {
            client,
            base_url,
            api_key,
        })
    }
}
