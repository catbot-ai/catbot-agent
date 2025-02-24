use chrono::{DateTime, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictionOutput {
    pub summary: Summary,
    pub long_signals: Vec<PredictedLongShortSignal>,
    pub short_signals: Vec<PredictedLongShortSignal>,
    // pub price_prediction_graph_5m: Vec<PricePredictionPoint5m>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RefinedPredictionOutput {
    pub timestamp: i64,
    pub local_datetime: String,
    pub summary: Summary,
    pub long_signals: Vec<LongShortSignal>,
    pub short_signals: Vec<LongShortSignal>,
    // pub price_prediction_graph_5m: Vec<PricePredictionPoint5m>,
}

pub struct PredictionOutputWithTimeStampBuilder {
    pub gemini_response: PredictionOutput,
    pub timezone: Tz, // Store the timezone here.
}

impl PredictionOutputWithTimeStampBuilder {
    pub fn new(gemini_response: PredictionOutput, timezone: Tz) -> Self {
        PredictionOutputWithTimeStampBuilder {
            gemini_response,
            timezone,
        }
    }

    pub fn build(self) -> RefinedPredictionOutput {
        let now_utc: DateTime<Utc> = Utc::now();
        let now_local = now_utc.with_timezone(&self.timezone);
        let iso_local = now_local.to_rfc3339();

        let long_signals = self
            .gemini_response
            .long_signals
            .into_iter()
            .map(LongShortSignal::from)
            .collect();

        let short_signals = self
            .gemini_response
            .short_signals
            .into_iter()
            .map(LongShortSignal::from)
            .collect();

        RefinedPredictionOutput {
            timestamp: now_utc.timestamp_millis(),
            local_datetime: iso_local,
            summary: self.gemini_response.summary,
            long_signals,
            short_signals,
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
    pub technical_resistance_4h: f64,
    pub technical_support_4h: f64,
    pub top_bids_price_amount: Vec<(String, f64)>,
    pub top_asks_price_amount: Vec<(String, f64)>,
    pub detail: String,
    pub suggestion: String,
    pub vibe: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictedLongShortSignal {
    pub symbol: String,
    pub confidence: f64,
    pub current_price: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub timeframe: String,
    pub target_datetime: String,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LongShortSignal {
    pub symbol: String,
    pub confidence: f64,
    pub current_price: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub timeframe: String,
    pub target_datetime: String,
    pub target_local_datetime: String,
    pub rationale: String,
}

impl From<PredictedLongShortSignal> for LongShortSignal {
    fn from(signal: PredictedLongShortSignal) -> Self {
        let utc_datetime = DateTime::parse_from_rfc3339(&signal.target_datetime)
            .expect("Failed to parse datetime");
        let tokyo_datetime: DateTime<Tz> = utc_datetime.with_timezone(&Tokyo);
        let target_local_datetime = tokyo_datetime.to_rfc3339();

        LongShortSignal {
            symbol: signal.symbol,
            confidence: signal.confidence,
            current_price: signal.current_price,
            entry_price: signal.entry_price,
            target_price: signal.target_price,
            stop_loss: signal.stop_loss,
            timeframe: signal.timeframe,
            target_datetime: signal.target_datetime,
            target_local_datetime,
            rationale: signal.rationale,
        }
    }
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
