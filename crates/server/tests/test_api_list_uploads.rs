use anyhow::Context;
use parcel_shared::types::api::ApiUploadsResponse;
use poem::{http::StatusCode, web::headers::Authorization};
use sqlx::SqlitePool;

mod utils;

#[test_log::test(sqlx::test(
    migrator = "parcel_model::migration::MIGRATOR",
    fixtures("api_tests.sql")
))]
async fn test_api_list_uploads(pool: SqlitePool) -> anyhow::Result<()> {
    let (_, client) = utils::create_test_client(&pool)
        .await
        .context("failed to create test client")?;

    let response = client
        .get("/api/1.0/uploads")
        .typed_header(
            Authorization::bearer(utils::TEST_API_KEY_1)
                .context("failed to create 'Authorization' header")?,
        )
        .send()
        .await;

    response.assert_status(StatusCode::OK);
    response.assert_content_type("application/json; charset=utf-8");

    let response_body = response.json().await.value().deserialize::<ApiUploadsResponse>();

    assert_eq!(response_body.offset, 0);
    assert_eq!(response_body.total, 2);
    assert_eq!(response_body.total_size, 2048 + 1024);

    Ok(())
}
