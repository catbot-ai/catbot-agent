use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PredictionOutput {
    pub summary: Summary,
    pub buy_signals: Vec<Signal>,
    pub sell_signals: Vec<Signal>,
    pub price_prediction_graph: Vec<PricePoint>,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Summary {
    pub title: String,
    pub detail: String,
    pub vibe: Option<String>,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Signal {
    pub price: f64,
    pub amount_usd: f64,
    pub amount_sol: f64,
    pub pair: String,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PricePoint {
    pub minute: u32,
    pub price: f64,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, serde_json::Value>,
}
