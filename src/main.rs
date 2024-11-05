mod config;
mod constants;
mod db;
mod models;
mod telegram;
mod facebook;
mod worker;

use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load configuration
    let config = config::Config::from_env()?;
    
    // Initialize database connection
    let db = Arc::new(tokio::sync::Mutex::new(db::Database::new(&config.database_url).await?));
    
    // Get Facebook accounts (now includes Telegram config)
    let facebook_accounts = db.lock().await.get_facebook_accounts().await?;
    
    // Get thresholds from database
    let thresholds = worker::AdMetricsThresholds::from_db(&db).await?;

    // Initialize worker
    let worker = worker::AdWorker::new(
        db,
        thresholds,
    );

    // Run the worker
    worker.run(facebook_accounts).await?;

    Ok(())
} 