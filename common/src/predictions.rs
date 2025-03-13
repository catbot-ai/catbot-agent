use jup_sdk::perps::{PerpsPosition, Side};

use chrono::{DateTime, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use serde::{Deserialize, Deserializer, Serialize};

use crate::Kline;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PredictionOutput {
    Suggestions(RefinedSuggestionOutput),
    GraphPredictions(RefinedGraphPredictionOutput),
}

pub trait Refinable {
    type Refined;
    fn refine(self, timezone: Tz, model_name: &str, prompt_hash: &str) -> Self::Refined;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct GraphPredictionOutput {
    pub signals: Vec<PredictedLongShortSignal>,
    pub klines: Vec<Kline>,
}

pub struct GraphPredictionOutputWithTimeStampBuilder {
    pub graph_response: GraphPredictionOutput,
    pub timezone: Tz,
}

impl GraphPredictionOutputWithTimeStampBuilder {
    pub fn new(graph_response: GraphPredictionOutput, timezone: Tz) -> Self {
        GraphPredictionOutputWithTimeStampBuilder {
            graph_response,
            timezone,
        }
    }

    pub fn build(self, model_name: &str, prompt_hash: &str) -> RefinedGraphPredictionOutput {
        let model_name = model_name.to_owned();
        let prompt_hash = prompt_hash.to_owned();

        let now_utc = Utc::now();
        let now_local = now_utc.with_timezone(&self.timezone);
        let iso_local = now_local.to_rfc3339();

        let iso_utc = now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        println!("🔥 self.graph_response:{:#?}", self.graph_response.clone());

        let signals = self
            .graph_response
            .signals
            .into_iter()
            .map(LongShortSignal::from)
            .collect();

        // Convert Vec<Kline> to Vec<Vec<KlineValue>>
        let klines = self
            .graph_response
            .klines
            .into_iter()
            .map(|kline| kline.to_kline_values())
            .collect();

        let timestamp = now_utc.timestamp_millis();

        RefinedGraphPredictionOutput {
            timestamp,
            current_datetime: iso_utc,
            current_datetime_local: iso_local,
            signals,
            klines,
            model_name,
            prompt_hash,
        }
    }
}

impl Refinable for GraphPredictionOutput {
    type Refined = RefinedGraphPredictionOutput;
    fn refine(self, timezone: Tz, model_name: &str, prompt_hash: &str) -> Self::Refined {
        GraphPredictionOutputWithTimeStampBuilder::new(self, timezone)
            .build(model_name, prompt_hash)
    }
}

impl Kline {
    fn to_kline_values(&self) -> Vec<KlineValue> {
        vec![
            KlineValue::Int64(self.open_time),
            KlineValue::String(self.open_price.clone()),
            KlineValue::String(self.high_price.clone()),
            KlineValue::String(self.low_price.clone()),
            KlineValue::String(self.close_price.clone()),
            KlineValue::String(self.volume.clone()),
            KlineValue::Int64(self.close_time),
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum KlineValue {
    Int64(i64),     // For signed integers (e.g., timestamps)
    UInt64(u64),    // For unsigned integers (if needed)
    String(String), // For prices and volumes
    Float64(f64),   // For floating-point numbers (optional)
    UInt32(u32),    // For smaller unsigned integers (e.g., number of trades)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RefinedGraphPredictionOutput {
    pub timestamp: i64,
    pub current_datetime: String,
    pub current_datetime_local: String,
    pub signals: Vec<LongShortSignal>,
    pub klines: Vec<Vec<KlineValue>>,
    // Stats
    pub model_name: String,
    pub prompt_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RefinedGraphPredictionResponse {
    pub timestamp: i64,
    pub current_datetime: String,
    pub current_datetime_local: String,
    pub signals: Vec<LongShortSignal>,
    pub klines: Vec<Kline>,
    // Stats
    pub model_name: String,
    pub prompt_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SuggestionOutput {
    pub summary: Summary,
    pub signals: Vec<PredictedLongShortSignal>,
    pub positions: Option<Vec<PredictedPosition>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RefinedSuggestionOutput {
    pub timestamp: i64,
    pub current_datetime: String,
    pub current_datetime_local: String,
    pub summary: Summary,
    pub signals: Vec<LongShortSignal>,
    pub positions: Option<Vec<PredictedPosition>>,
    // Stats
    pub model_name: String,
    pub prompt_hash: String,
}

pub struct SuggestionOutputWithTimeStampBuilder {
    pub gemini_response: SuggestionOutput,
    pub timezone: Tz, // Store the timezone here.
}

impl SuggestionOutputWithTimeStampBuilder {
    pub fn new(gemini_response: SuggestionOutput, timezone: Tz) -> Self {
        SuggestionOutputWithTimeStampBuilder {
            gemini_response,
            timezone,
        }
    }

    pub fn build(self, model_name: &str, prompt_hash: &str) -> RefinedSuggestionOutput {
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

        RefinedSuggestionOutput {
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

impl Refinable for SuggestionOutput {
    type Refined = RefinedSuggestionOutput;
    fn refine(self, timezone: Tz, model_name: &str, prompt_hash: &str) -> Self::Refined {
        SuggestionOutputWithTimeStampBuilder::new(self, timezone).build(model_name, prompt_hash)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Summary {
    pub price: f64,
    pub upper_bound: f64,
    pub lower_bound: f64,
    pub technical_resistance_4h: f64,
    pub technical_support_4h: f64,
    #[serde(deserialize_with = "deserialize_vec_tuples")]
    pub top_bids_price_amount: Vec<Vec<f64>>,
    #[serde(deserialize_with = "deserialize_vec_tuples")]
    pub top_asks_price_amount: Vec<Vec<f64>>,
    pub vibe: String,
    pub detail: String,
    pub suggestion: String,
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
    pub direction: String,
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
    pub direction: String,
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
            direction: signal.direction,
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
    pub rationale: String,
    pub confidence: f64, // Confidence score between 0.0 and 1.0
}

impl From<PerpsPosition> for PredictedPosition {
    fn from(perps: PerpsPosition) -> Self {
        PredictedPosition {
            // From ai
            new_target_price: None,
            new_stop_loss: None,
            suggestion: "n/a".to_string(),
            rationale: "n/a".to_string(),
            confidence: perps.confidence,
            // Base
            ..perps.into()
        }
    }
}
