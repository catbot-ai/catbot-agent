use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PredictionOutput {
    pub summary: Summary,
    pub long_signals: Vec<LongSignal>,
    pub short_signals: Vec<ShortSignal>,
    pub price_prediction_graph_5m: Vec<PricePredictionPoint5m>,
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
pub struct LongSignal {
    pub symbol: String,
    pub amount: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub rationale: String,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ShortSignal {
    pub symbol: String,
    pub amount: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub rationale: String,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PricePredictionPoint5m {
    pub price: f64,
    pub upper: f64,
    pub lower: f64,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, serde_json::Value>,
}
