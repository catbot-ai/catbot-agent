use jup_sdk::perps::{PerpsPosition, Side};

use chrono::{DateTime, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictionOutput {
    pub summary: Summary,
    pub signals: Vec<PredictedLongShortSignal>,
    pub positions: Option<Vec<PredictedPosition>>,
    // pub price_prediction_graph_5m: Vec<PricePredictionPoint5m>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RefinedPredictionOutput {
    pub timestamp: i64,
    pub current_datetime: String,
    pub current_datetime_local: String,
    pub summary: Summary,
    pub signals: Vec<LongShortSignal>,
    pub positions: Option<Vec<PredictedPosition>>,
    // Stats
    pub model_name: String,
    pub prompt_hash: String,
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

    pub fn build(self, model_name: &str, prompt_hash: &str) -> RefinedPredictionOutput {
        let model_name = model_name.to_owned();
        let prompt_hash = prompt_hash.to_owned();

        let now_utc = Utc::now();
        let now_local = now_utc.with_timezone(&self.timezone);
        let iso_local = now_local.to_rfc3339();

        let iso_utc = now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let signals = self
            .gemini_response
            .signals
            .into_iter()
            .map(LongShortSignal::from)
            .collect();

        let positions = self.gemini_response.positions.or(Some(vec![]));
        let timestamp = now_utc.timestamp_millis();

        RefinedPredictionOutput {
            timestamp,
            current_datetime: iso_utc,
            current_datetime_local: iso_local,
            summary: self.gemini_response.summary,
            signals,
            positions,
            model_name,
            prompt_hash,
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
    #[serde(deserialize_with = "deserialize_vec_tuples")]
    pub top_bids_price_amount: Vec<Vec<f64>>,
    #[serde(deserialize_with = "deserialize_vec_tuples")]
    pub top_asks_price_amount: Vec<Vec<f64>>,
    pub detail: String,
    pub suggestion: String,
    pub vibe: Option<String>,
}

fn deserialize_vec_tuples<'de, D>(deserializer: D) -> Result<Vec<Vec<f64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let tuples: Vec<(f64, f64)> = Deserialize::deserialize(deserializer)?;
    Ok(tuples.into_iter().map(|(a, b)| vec![a, b]).collect())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictedLongShortSignal {
    pub side: String,
    pub symbol: String,
    pub confidence: f64,
    pub current_price: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub timeframe: String,
    pub entry_datetime: String,
    pub target_datetime: String,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LongShortSignal {
    pub side: String,
    pub symbol: String,
    pub confidence: f64,
    pub current_price: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub timeframe: String,
    pub entry_datetime: String,
    pub target_datetime: String,
    pub entry_datetime_local: String,
    pub target_datetime_local: String,
    pub rationale: String,
}

impl From<PredictedLongShortSignal> for LongShortSignal {
    fn from(signal: PredictedLongShortSignal) -> Self {
        println!("{signal:#?}");
        let utc_datetime = DateTime::parse_from_rfc3339(&signal.target_datetime)
            .expect("Failed to parse datetime");
        let tokyo_datetime: DateTime<Tz> = utc_datetime.with_timezone(&Tokyo);
        let target_datetime_local = tokyo_datetime.to_rfc3339();

        let utc_datetime =
            DateTime::parse_from_rfc3339(&signal.entry_datetime).expect("Failed to parse datetime");
        let tokyo_datetime: DateTime<Tz> = utc_datetime.with_timezone(&Tokyo);
        let entry_datetime_local = tokyo_datetime.to_rfc3339();

        LongShortSignal {
            side: signal.side,
            symbol: signal.symbol,
            confidence: signal.confidence,
            current_price: signal.current_price,
            entry_price: signal.entry_price,
            target_price: signal.target_price,
            stop_loss: signal.stop_loss,
            timeframe: signal.timeframe,
            target_datetime: signal.target_datetime,
            entry_datetime: signal.entry_datetime,
            entry_datetime_local,
            target_datetime_local,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictedPosition {
    pub side: Side,                // Position side: long or short
    pub market_mint: String,       // So11111111111111111111111111111111111111112
    pub collateral_mint: String,   // EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
    pub entry_price: f64,          // Entry price of the position
    pub leverage: f64,             // Leverage used for the position
    pub liquidation_price: f64,    // Liquidation price of the position
    pub pnl_after_fees_usd: f64,   // Profit/loss after fees in USD
    pub value: f64,                // Current position value in USD
    pub target_price: Option<f64>, // Optional current target price in USD
    pub stop_loss: Option<f64>,    // Optional current stop loss in USD
    // From ai
    pub new_target_price: Option<f64>, //  Optional suggested new target price
    pub new_stop_loss: Option<f64>,    // Optional suggested new stop loss
    pub suggestion: String, // Suggestion for this position. e.g. "Hold short position. Consider increasing position at 138.5 with stop loss at 140.5 and taking profit at 135."
    pub confidence: f64,    // Confidence score between 0.0 and 1.0
}

impl From<PerpsPosition> for PredictedPosition {
    fn from(perps: PerpsPosition) -> Self {
        PredictedPosition {
            // From ai
            new_target_price: None,
            new_stop_loss: None,
            suggestion: "n/a".to_string(),
            confidence: perps.confidence,
            // Base
            side: perps.side,
            market_mint: perps.market_mint,
            collateral_mint: perps.collateral_mint,
            entry_price: perps.entry_price,
            leverage: perps.leverage,
            liquidation_price: perps.liquidation_price,
            pnl_after_fees_usd: perps.pnl_after_fees_usd,
            value: perps.value,
            target_price: perps.target_price,
            stop_loss: perps.stop_loss,
        }
    }
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
