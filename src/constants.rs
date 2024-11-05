// API Versions
pub const FACEBOOK_API_VERSION: &str = "v20.0";

// API Base URLs
pub const FACEBOOK_BASE_URL: &str = "https://graph.facebook.com";
pub const TELEGRAM_BASE_URL: &str = "https://api.telegram.org";

// Content Types
pub const CONTENT_TYPE_HEADER: &str = "Content-Type";
pub const CONTENT_TYPE_JSON: &str = "application/json";

// Worker Settings
pub const WORKER_INTERVAL_SECS: u64 = 1800; // 30 minutes

// Facebook API Fields
pub const FB_AD_FIELDS: &str = "id,name,status,effective_status,insights.fields(impressions,reach,clicks,spend,cost_per_action_type,actions)";
pub const FB_ACCOUNT_FIELDS: &str = "balance,name,id,account_status,currency"; 