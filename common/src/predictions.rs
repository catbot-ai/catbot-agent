use chrono::{DateTime, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictionOutput {
    pub summary: Summary,
    pub long_signals: Vec<LongSignal>,
    pub short_signals: Vec<ShortSignal>,
    // pub price_prediction_graph_5m: Vec<PricePredictionPoint5m>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictionOutputWithTimeStamp {
    pub timestamp: i64,
    pub local_datetime: String,
    pub summary: Summary,
    pub long_signals: Vec<LongSignal>,
    pub short_signals: Vec<ShortSignal>,
    // pub price_prediction_graph_5m: Vec<PricePredictionPoint5m>,
}

fn convert_to_tokyo_iso(datetime_str: &str) -> String {
    let utc_datetime =
        DateTime::parse_from_rfc3339(datetime_str).expect("Failed to parse datetime");
    let tokyo_datetime: DateTime<Tz> = utc_datetime.with_timezone(&Tokyo);
    tokyo_datetime.to_rfc3339()
}

impl From<PredictionOutput> for PredictionOutputWithTimeStamp {
    fn from(gemini_response: PredictionOutput) -> Self {
        let now_utc: DateTime<Utc> = Utc::now();
        let iso_tokyo = convert_to_tokyo_iso(&now_utc.to_rfc3339());

        let long_signals = gemini_response
            .long_signals
            .into_iter()
            .map(|mut signal| {
                signal.local_target_datetime = convert_to_tokyo_iso(&signal.target_datetime);
                signal
            })
            .collect();

        let short_signals = gemini_response
            .short_signals
            .into_iter()
            .map(|mut signal| {
                signal.local_target_datetime = convert_to_tokyo_iso(&signal.target_datetime);
                signal
            })
            .collect();

        PredictionOutputWithTimeStamp {
            timestamp: now_utc.timestamp_millis(),
            local_datetime: iso_tokyo,
            summary: gemini_response.summary,
            long_signals,
            short_signals,
            // price_prediction_graph_5m: gemini_response.price_prediction_graph_5m,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Summary {
    pub title: String,
    pub price: f64,
    pub upper_bound: f64,
    pub lower_bound: f64,
    pub top_3_supports: [f64; 3],
    pub top_3_resistances: [f64; 3],
    pub technical_resistance_4h: f64,
    pub technical_support_4h: f64,
    pub detail: String,
    pub suggestion: String,
    pub vibe: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LongSignal {
    pub symbol: String,
    pub amount: f64,
    pub current_price: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub timeframe: String,
    pub target_datetime: String,
    pub local_target_datetime: String,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ShortSignal {
    pub symbol: String,
    pub amount: f64,
    pub current_price: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub timeframe: String,
    pub target_datetime: String,
    pub local_target_datetime: String,
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

// TODO: separated call for price prediction
// "price_prediction_graph_5m": [
//     {{
//       "price": "number",            // Start with current {symbol} price and so on.
//       "upper_bound": "number",      // Start with current {symbol} upper bound and so on.
//       "lower_bound": "number"       // Start with current {symbol} lower bound and so on.
//       "first_resistance": "number"  // Start with current {symbol} first significant amount of resistance and so on.
//       "first_support": "number"     // Start with current {symbol} first significant amount of support and so on.
//       "second_resistance": "number" // Start with current {symbol} second significant amount of resistance and so on.
//       "second_support": "number"    // Start with current {symbol} second significant amount of support and so on.
//     }}
//   ]

// Provide a price prediction graph with 5-minute intervals for the next 4 hours.
// Include upper and lower bounds. Format this in the price_prediction_graph_5m field.
