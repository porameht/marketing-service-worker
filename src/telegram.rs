use crate::models::{AdAccountMetrics, TelegramConfig};
use reqwest::Client;
use serde_json::json;
use std::error::Error;
use thiserror::Error;
use crate::constants::{
    TELEGRAM_BASE_URL, 
    CONTENT_TYPE_HEADER, 
    CONTENT_TYPE_JSON
};

#[derive(Error, Debug)]
pub enum TelegramError {
    #[error("Failed to send message: {0}")]
    SendError(String),
    #[error("API request failed: {0}")]
    RequestError(#[from] reqwest::Error),
}

pub struct TelegramNotifier {
    client: Client,
    base_url: String,
    chat_id: i64,
}

impl TelegramNotifier {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("{}/bot{}", TELEGRAM_BASE_URL, config.bot_token),
            chat_id: config.chat_id,
        }
    }

    pub async fn send_message(&self, message: &str) -> Result<(), TelegramError> {
        let url = format!("{}/sendMessage", self.base_url);
        
        let params = json!({
            "chat_id": self.chat_id,
            "text": message,
            "parse_mode": "HTML"
        });

        let response = self.client
            .post(&url)
            .header(CONTENT_TYPE_HEADER, CONTENT_TYPE_JSON)
            .json(&params)
            .send()
            .await
            .map_err(|e| TelegramError::RequestError(e))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TelegramError::SendError(error_text));
        }

        println!("Telegram message sent successfully");
        Ok(())
    }

    pub async fn send_metrics_alert(&self, metrics: &AdAccountMetrics) -> Result<(), TelegramError> {
        let message = format!(
            "📊 Ad Account Update: {}\n\
             💰 Spend: ${:.2}\n\
             👁 Impressions: {}\n\
             🖱 Clicks: {}\n\
             ✅ Conversions: {}\n",
            metrics.account_id, metrics.spend, metrics.impressions, metrics.clicks, metrics.conversions
        );

        self.send_message(&message).await
    }
} 