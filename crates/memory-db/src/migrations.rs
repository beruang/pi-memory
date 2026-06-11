use sqlx::PgPool;
use tracing::info;

pub async fn migrate(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(pool).await?;
    info!("Migrations complete.");
    Ok(())
}
