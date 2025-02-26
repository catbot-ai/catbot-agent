use jup_sdk::perps::PositionData;
use strum::Display;

use chrono::{DateTime, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use serde::{Deserialize, Serialize};
use strum::EnumString;

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

        let iso_utc = now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let signals = self
            .gemini_response
            .signals
            .into_iter()
            .map(LongShortSignal::from)
            .collect();

        let positions = self.gemini_response.positions.or(Some(vec![]));

        RefinedPredictionOutput {
            timestamp: now_utc.timestamp_millis(),
            current_datetime: iso_utc,
            current_datetime_local: iso_local,
            summary: self.gemini_response.summary,
            signals,
            positions,
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
    pub top_bids_price_amount: Vec<(f64, f64)>,
    pub top_asks_price_amount: Vec<(f64, f64)>,
    pub detail: String,
    pub suggestion: String,
    pub vibe: Option<String>,
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

#[derive(Clone, Serialize, Deserialize, Debug, EnumString, Display, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Side {
    Long,
    Short,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PerpsPosition {
    pub side: Side,              // Position side: long or short
    pub symbol: String,          // Trading pair symbol (e.g., "SOL")
    pub confidence: f64,         // Confidence score between 0.0 and 1.0
    pub entry_price: f64,        // Entry price of the position
    pub leverage: f64,           // Leverage used for the position
    pub liquidation_price: f64,  // Liquidation price of the position
    pub pnl_after_fees_usd: f64, // Profit/loss after fees in USD
    pub value: f64,              // Current position value in USD
}

impl From<PositionData> for PerpsPosition {
    fn from(position: PositionData) -> Self {
        let side = match position.side {
            jup_sdk::perps::Side::Long => Side::Long,
            jup_sdk::perps::Side::Short => Side::Short,
        };
        // Extract symbol from market_mint or use a default if parsing fails
        let symbol = position
            .market_mint
            .split("USDT")
            .next()
            .unwrap_or("UNKNOWN")
            .to_string();

        // Parse string fields into f64, defaulting to 0.0 or 1.0 if parsing fails
        let entry_price = position.entry_price.parse::<f64>().unwrap_or(0.0);
        let leverage = position.leverage.parse::<f64>().unwrap_or(1.0); // Default to 1x if invalid
        let liquidation_price = position.liquidation_price.parse::<f64>().unwrap_or(0.0);
        let confidence = 0.5; // Default value (could be computed elsewhere)
        let pnl_after_fees_usd = position.pnl_after_fees_usd.parse::<f64>().unwrap_or(0.0);
        let value = position.value.parse::<f64>().unwrap_or(0.0);

        PerpsPosition {
            side,
            symbol,
            confidence,
            entry_price,
            leverage,
            liquidation_price,
            pnl_after_fees_usd,
            value,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictedPosition {
    pub side: Side,                          // Position side: long or short
    pub symbol: String,                      // Trading pair symbol (e.g., "SOL")
    pub confidence: f64,                     // Confidence score between 0.0 and 1.0
    pub entry_price: f64,                    // Entry price of the position
    pub leverage: f64,                       // Leverage used for the position
    pub liquidation_price: f64,              // Liquidation price of the position
    pub pnl_after_fees_usd: f64,             // Profit/loss after fees in USD
    pub value: f64,                          // Current position value in USD
    pub suggested_target_price: Option<f64>, // Optional suggested target price
    pub suggested_stop_loss: Option<f64>,    // Optional suggested stop loss
    pub suggested_add_value: Option<f64>,    // Optional suggestion to add value
}

impl From<PerpsPosition> for PredictedPosition {
    fn from(perps: PerpsPosition) -> Self {
        PredictedPosition {
            side: perps.side,
            symbol: perps.symbol,
            confidence: perps.confidence,
            entry_price: perps.entry_price,
            leverage: perps.leverage,
            liquidation_price: perps.liquidation_price,
            pnl_after_fees_usd: perps.pnl_after_fees_usd,
            value: perps.value,
            suggested_target_price: None, // No equivalent in PerpsPosition
            suggested_stop_loss: None,    // No equivalent in PerpsPosition
            suggested_add_value: None,    // No equivalent in PerpsPosition
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
