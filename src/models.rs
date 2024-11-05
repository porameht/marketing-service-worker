use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FacebookAccount {
    pub id: i32,
    pub access_token: String,
    pub account_id: String,
    pub is_active: bool,
    pub interval: i32,  // Monitoring interval in minutes
    pub telegram_config: TelegramConfig, // Add telegram config to each account
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdAccountMetrics {
  pub account_id: String,
  pub spend: f64,
  pub impressions: i64,
  pub clicks: i64,
  pub conversions: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdThresholds {
    pub max_cost_per_action: f64,
} 