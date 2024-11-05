use dotenv::dotenv;
use std::env;
use thiserror::Error;
use url::Url;

#[derive(Debug)]
pub struct Config {
    pub database_url: String,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Environment variable not found: {0}")]
    MissingEnv(String),
    #[error("Invalid database URL: {0}")]
    InvalidDatabaseUrl(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingEnv("DATABASE_URL".to_string()))?;

        // Validate the URL format
        Url::parse(&database_url)
            .map_err(|e| ConfigError::InvalidDatabaseUrl(e.to_string()))?;

        Ok(Self { database_url })
    }
} 