use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Subscription {
    pub api_url: String,
    pub api_key: String,
    pub webhook_url: String,
    pub webhook_key: String,
}
