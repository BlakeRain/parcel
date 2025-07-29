use anyhow::Context;
use clap::Parser;
use parcel_server::{app::create_app, args::Args, env::Env};
use poem::test::TestClient;
use sqlx::SqlitePool;

pub async fn create_test_client(
    pool: &SqlitePool,
) -> anyhow::Result<(Env, TestClient<impl poem::Endpoint>)> {
    let args =
        Args::try_parse_from::<_, String>([]).context("failed to parse command line arguments")?;
    let env = Env::new_with_pool(&args, pool.clone())
        .await
        .context("failed to create environment")?;
    let (preview, _) = parcel_server::workers::previews::start_worker(env.clone())
        .await
        .context("failed to start preview worker")?;
    let app = create_app(env.clone(), preview, None).context("failed to create app")?;
    Ok((env, TestClient::new(app)))
}
