use crate::models::{FacebookAccount, TelegramConfig, AdThresholds, AdAccountMetrics};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use tokio_postgres::{Client, NoTls, Config};
use std::error::Error;
use thiserror::Error;
use std::str::FromStr;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    ConnectionError(#[from] tokio_postgres::Error),
    #[error("SSL error: {0}")]
    SslError(#[from] openssl::error::ErrorStack),
    #[error("Invalid connection string: {0}")]
    InvalidConnectionString(String),
}

pub struct Database {
    client: Client,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, DatabaseError> {
    // Configure SSL
        let mut builder = SslConnector::builder(SslMethod::tls())?;
        builder.set_verify(SslVerifyMode::NONE); // For development only, use proper verification in production
        let connector = MakeTlsConnector::new(builder.build());

        // Parse the connection config from URL
        let mut config = Config::from_str(database_url)
            .map_err(|e| DatabaseError::InvalidConnectionString(e.to_string()))?;

        // Connect with SSL
        let (client, connection) = config
          .connect_timeout(std::time::Duration::from_secs(5))
          .connect(connector)
          .await
          .map_err(DatabaseError::ConnectionError)?;
        
        // Spawn the connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {}", e);
            }
        });

        Ok(Self { client })
    }

    pub async fn get_facebook_accounts(&self) -> Result<Vec<FacebookAccount>, DatabaseError> {
        let rows = self.client.query(
            "SELECT 
                fa.id, 
                fa.access_token, 
                fa.account_id, 
                fa.is_active, 
                fa.interval,
                tc.bot_token,
                tc.chat_id
             FROM facebook_accounts fa
             INNER JOIN telegram_config tc ON fa.telegram_config_id = tc.id
             WHERE fa.is_active = true",
            &[],
        ).await?;

        let accounts = rows
            .iter()
            .map(|row| FacebookAccount {
                id: row.get(0),
                access_token: row.get(1),
                account_id: row.get(2),
                is_active: row.get(3),
                interval: row.get(4),
                telegram_config: TelegramConfig {
                    bot_token: row.get(5),
                    chat_id: row.get(6),
                },
            })
            .collect();

        Ok(accounts)
    }

    pub async fn update_metrics(&self, metrics: &AdAccountMetrics) -> Result<(), DatabaseError> {
        self.client.execute(
            "INSERT INTO account_metrics (account_id, spend, impressions, clicks, conversions, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
            &[
                &metrics.account_id,
                &metrics.spend,
                &metrics.impressions,
                &metrics.clicks,
                &metrics.conversions,
            ],
        ).await?;

        Ok(())
    }

    pub async fn get_ad_thresholds(&self) -> Result<AdThresholds, DatabaseError> {
        let row = self.client.query_one(
            "SELECT max_cost_per_action FROM ad_thresholds LIMIT 1",
            &[],
        ).await?;

        Ok(AdThresholds {
            max_cost_per_action: row.get(0),
        })
    }
} 