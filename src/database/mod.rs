pub mod hybrid;
pub mod schema;
pub mod service;

use sea_orm::Database as SeaOrmDatabase;
use sea_orm::DbConn;

/// Initialize database connection
pub async fn init_database(database_url: &str) -> anyhow::Result<DbConn> {
    let db = SeaOrmDatabase::connect(database_url).await?;
    tracing::info!("Database connected successfully");
    Ok(db)
}
