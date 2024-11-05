use crate::facebook::{FacebookAPI, AdData};
use crate::models::{FacebookAccount, AdThresholds};
use crate::telegram::TelegramNotifier;
use crate::db::Database;
use crate::constants::WORKER_INTERVAL_SECS;
use std::error::Error;
use std::sync::Arc;
use tokio::time::{self, Duration};

pub const WORKER_INTERVAL: Duration = Duration::from_secs(WORKER_INTERVAL_SECS);

#[derive(Debug, Clone)]
pub struct AdMetricsThresholds {
    pub max_cost_per_action: f64,
}

impl AdMetricsThresholds {
  pub async fn from_db(db: &Arc<tokio::sync::Mutex<Database>>) -> Result<Self, Box<dyn Error>> {
      let thresholds = db.lock().await.get_ad_thresholds().await?;
      Ok(Self {
          max_cost_per_action: thresholds.max_cost_per_action,
      })
  }
}

pub struct AdWorker {
    db: Arc<tokio::sync::Mutex<Database>>,
    thresholds: AdMetricsThresholds,
}

impl AdWorker {
    pub fn new(
        db: Arc<tokio::sync::Mutex<Database>>,
        thresholds: AdMetricsThresholds,
    ) -> Self {
        Self {
            db,
            thresholds,
        }
    }

    pub async fn process_account(&self, account: &FacebookAccount) -> Result<(), Box<dyn Error>> {
        // Create Telegram notifier for this specific account
        let telegram = TelegramNotifier::new(account.telegram_config.clone());
        
        let fb_api = FacebookAPI::new(account.access_token.clone(), account.account_id.clone());
        
        // Get ads for the account
        let ads = fb_api.get_ads().await?;
        
        if ads.is_empty() {
            telegram.send_message(&format!(
                "🔍 Account {} has no active ads", 
                account.account_id
            )).await?;
            return Ok(());
        }

        // Check account balance
        let balance = fb_api.get_ad_account_balance().await?;
        telegram.send_message(&format!(
            "💰 Account balance {}: {}", 
            account.account_id, 
            balance.available_funds
        )).await?;

        let mut all_paused = true;
        let mut messages = Vec::new();

        for ad in ads {
            self.process_ad(&fb_api, &ad, &mut all_paused, &mut messages).await?;
        }

        if all_paused {
            messages.push(format!("🚨 Account {} all ads are paused", account.account_id));
        }

        // Send combined message
        let combined_message = messages.join("\n");
        telegram.send_message(&combined_message).await?;

        Ok(())
    }

    async fn process_ad(
        &self,
        fb_api: &FacebookAPI,
        ad: &AdData,
        all_paused: &mut bool,
        messages: &mut Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        if ad.effective_status == "DISAPPROVED" {
            messages.push(format!("❌ Ad disapproved: {} waiting for deletion", ad.name));
            return Ok(());
        }

        if ad.effective_status != "CAMPAIGN_PAUSED" {
            *all_paused = false;
        }

        // Process cost per action
        let cost_per_action = self.get_cost_per_action(ad);
        
        // Determine if ad status needs to change
        let new_status = if self.should_close_ad(ad, cost_per_action) {
            Some("PAUSED")
        } else if self.should_open_ad(ad, cost_per_action) {
            Some("ACTIVE")
        } else {
            None
        };

        // Update ad status if needed
        if let Some(status) = new_status {
            fb_api.update_ad_status(&ad.name, status).await?;
            messages.push(format!("🧠 Updated ad status: {} to {}", ad.name, status));
        }

        // Add status message
        let status_msg = if ad.status == "PAUSED" || ad.effective_status == "CAMPAIGN_PAUSED" {
            format!("❌ Ad paused: {}:💰{}", ad.name, cost_per_action)
        } else {
            format!("🟢 Ad active: {}:💰{}", ad.name, cost_per_action)
        };
        messages.push(status_msg);

        Ok(())
    }

    fn should_close_ad(&self, ad: &AdData, cost_per_action: f64) -> bool {
        ad.status == "ACTIVE" 
            && ad.effective_status == "ACTIVE" 
            && cost_per_action > self.thresholds.max_cost_per_action
    }

    fn should_open_ad(&self, ad: &AdData, cost_per_action: f64) -> bool {
        ad.status == "PAUSED" 
            && ad.effective_status == "PAUSED" 
            && cost_per_action < self.thresholds.max_cost_per_action
    }

    fn get_cost_per_action(&self, ad: &AdData) -> f64 {
        ad.cost_per_action_type
            .iter()
            .find(|action| action.action_type == "offsite_conversion.fb_pixel_custom")
            .and_then(|action| action.value.parse::<f64>().ok())
            .unwrap_or(0.0)
    }

    pub async fn run(&self, facebook_accounts: Vec<FacebookAccount>) -> Result<(), Box<dyn Error>> {
      loop {
          for account in &facebook_accounts {
              if let Err(e) = self.process_account(account).await {
                  eprintln!("Error processing account {}: {}", account.account_id, e);
                  let telegram = TelegramNotifier::new(account.telegram_config.clone());
                  telegram.send_message(&format!(
                      "🚨 Error in account {}: {}", 
                      account.account_id, 
                      e
                  )).await?;
              }
          }
  
          time::sleep(WORKER_INTERVAL).await;
      }
  }
} 