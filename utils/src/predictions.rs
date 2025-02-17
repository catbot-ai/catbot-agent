use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictionOutput {
    pub summary: Summary,
    pub long_signals: Vec<LongSignal>,
    pub short_signals: Vec<ShortSignal>,
    pub price_prediction_graph_5m: Vec<PricePredictionPoint5m>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictionOutputWithTimeStamp {
    pub timestamp: i64,
    pub summary: Summary,
    pub long_signals: Vec<LongSignal>,
    pub short_signals: Vec<ShortSignal>,
    pub price_prediction_graph_5m: Vec<PricePredictionPoint5m>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Summary {
    pub title: String,
    pub current_price: f64,
    pub upper_bound: f64,
    pub lower_bound: f64,
    pub first_resistance: f64,
    pub first_support: f64,
    pub second_resistance: f64,
    pub second_support: f64,
    pub detail: String,
    pub suggestion: String,
    pub vibe: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LongSignal {
    pub symbol: String,
    pub amount: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub timeframe: String,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ShortSignal {
    pub symbol: String,
    pub amount: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub timeframe: String,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PricePredictionPoint5m {
    pub price: f64,
    pub upper_bound: f64,
    pub lower_bound: f64,
    pub first_resistance: f64,
    pub first_support: f64,
    pub second_resistance: f64,
    pub second_support: f64,
}
