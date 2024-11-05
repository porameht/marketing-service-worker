use crate::models::AdAccountMetrics;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use thiserror::Error;
use crate::constants::{
    FACEBOOK_API_VERSION, 
    FACEBOOK_BASE_URL, 
    FB_AD_FIELDS, 
    FB_ACCOUNT_FIELDS
};

#[derive(Error, Debug)]
pub enum FacebookApiError {
    #[error("API request failed: {0}")]
    RequestFailed(String),
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdData {
    pub id: String,
    pub name: String,
    pub status: String,
    pub effective_status: String,
    pub insights: Option<AdInsights>,
    pub cost_per_action_type: Vec<CostPerAction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdInsights {
    pub impressions: i64,
    pub reach: i64,
    pub clicks: i64,
    pub spend: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CostPerAction {
    pub action_type: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountBalance {
    pub name: String,
    pub id: String,
    pub status: String,
    pub currency: String,
    pub available_funds: String,
}

pub struct FacebookAPI {
    client: Client,
    access_token: String,
    account_id: String,
    base_url: String,
}

impl FacebookAPI {
    pub fn new(access_token: String, account_id: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
            account_id,
            base_url: format!("{}/{}", FACEBOOK_BASE_URL, FACEBOOK_API_VERSION),
        }
    }

    pub async fn get_ads(&self) -> Result<Vec<AdData>, FacebookApiError> {
        let url = format!("{}/act_{}/ads", self.base_url, self.account_id);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("access_token", &self.access_token),
                ("fields", &FB_AD_FIELDS.to_string()),
            ])
            .send()
            .await
            .map_err(|e| FacebookApiError::RequestFailed(e.to_string()))?;

        let ads: serde_json::Value = response
            .json()
            .await
            .map_err(|e| FacebookApiError::InvalidResponse(e.to_string()))?;

        let mut result = Vec::new();
        
        if let Some(data) = ads["data"].as_array() {
            for ad in data {
                let insights = ad.get("insights")
                    .and_then(|i| i.get("data"))
                    .and_then(|d| d.get(0))
                    .unwrap_or(&serde_json::Value::Null);

                let cost_per_action = insights
                    .get("cost_per_action_type")
                    .and_then(|actions| {
                        actions.as_array()?.iter().find(|action| {
                            action.get("action_type")
                                .and_then(|t| t.as_str())
                                == Some("offsite_conversion.fb_pixel_custom")
                        })
                    })
                    .and_then(|action| action.get("value"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string();

                result.push(AdData {
                    id: ad["id"].as_str().unwrap_or("").to_string(),
                    name: ad["name"].as_str().unwrap_or("").to_string(),
                    status: ad["status"].as_str().unwrap_or("").to_string(),
                    effective_status: ad["effective_status"].as_str().unwrap_or("").to_string(),
                    insights: Some(AdInsights {
                        impressions: insights.get("impressions").and_then(|v| v.as_i64()).unwrap_or(0),
                        reach: insights.get("reach").and_then(|v| v.as_i64()).unwrap_or(0),
                        clicks: insights.get("clicks").and_then(|v| v.as_i64()).unwrap_or(0),
                        spend: insights.get("spend").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    }),
                    cost_per_action_type: vec![CostPerAction {
                        action_type: "offsite_conversion.fb_pixel_custom".to_string(),
                        value: cost_per_action,
                    }],
                });
            }
        }

        Ok(result)
    }

    pub async fn update_ad_status(&self, ad_name: &str, status: &str) -> Result<Vec<AdData>, FacebookApiError> {
        let status = match status.to_lowercase().as_str() {
            "a" | "active" => "ACTIVE",
            "p" | "paused" => "PAUSED",
            _ => return Err(FacebookApiError::RequestFailed("Invalid status value".to_string())),
        };

        let url = format!("{}/act_{}/ads", self.base_url, self.account_id);
        let response = self.client
            .get(&url)
            .query(&[
                ("access_token", &self.access_token),
                ("fields", &"id,name,status,effective_status".to_string()),
                ("filtering", &format!("[{{'field':'name','operator':'CONTAIN','value':'{}'}}]", ad_name)),
            ])
            .send()
            .await
            .map_err(|e| FacebookApiError::RequestFailed(e.to_string()))?;

        let ads: serde_json::Value = response.json().await
            .map_err(|e| FacebookApiError::InvalidResponse(e.to_string()))?;

        let mut updated_ads = Vec::new();

        if let Some(data) = ads["data"].as_array() {
            for ad in data {
                let ad_id = ad["id"].as_str().unwrap_or("");
                if ad["effective_status"] != status {
                    // Update ad status
                    let update_url = format!("{}/{}", self.base_url, ad_id);
                    let _update_response = self.client
                        .post(&update_url)
                        .query(&[
                            ("access_token", &self.access_token),
                            ("status", &status.to_string()),
                        ])
                        .send()
                        .await
                        .map_err(|e| FacebookApiError::RequestFailed(e.to_string()))?;

                    updated_ads.push(AdData {
                        id: ad_id.to_string(),
                        name: ad["name"].as_str().unwrap_or("").to_string(),
                        status: status.to_string(),
                        effective_status: status.to_string(),
                        insights: None,
                        cost_per_action_type: vec![],
                    });
                }
            }
        }

        Ok(updated_ads)
    }

    pub async fn get_ad_account_balance(&self) -> Result<AccountBalance, FacebookApiError> {
        let url = format!("{}/act_{}", self.base_url, self.account_id);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("access_token", &self.access_token),
                ("fields", &FB_ACCOUNT_FIELDS.to_string()),
            ])
            .send()
            .await
            .map_err(|e| FacebookApiError::RequestFailed(e.to_string()))?;

        let account: serde_json::Value = response
            .json()
            .await
            .map_err(|e| FacebookApiError::InvalidResponse(e.to_string()))?;

        let balance = account["balance"].as_f64().unwrap_or(0.0);
        let currency = account["currency"].as_str().unwrap_or("THB");
        
        let balance_in_currency = if currency == "THB" {
            balance / 100.0  // Convert satang to baht
        } else {
            balance / 100.0  // Default conversion
        };

        Ok(AccountBalance {
            name: account["name"].as_str().unwrap_or("Unknown").to_string(),
            id: account["id"].as_str().unwrap_or("Unknown").to_string(),
            status: if account["account_status"].as_i64().unwrap_or(0) == 1 {
                "Active".to_string()
            } else {
                "Inactive".to_string()
            },
            currency: currency.to_string(),
            available_funds: format!("฿{:.2}", balance_in_currency),
        })
    }
} 