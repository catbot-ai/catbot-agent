use anyhow::Context;
use chrono::{DateTime, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use jup_sdk::{
    perps::{PerpsPosition, Side},
    token_registry::get_by_address,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Number as JsonNumber, Value as JsonValue};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PredictionOutput {
    TradingPredictions(RefinedTradingPrediction),
    GraphPredictions(RefinedGraphPrediction),
}

pub trait Refinable {
    type Refined;
    fn refine(
        self,
        timezone: Tz,
        model_name: &str,
        prompt_hash: &str,
        context: Option<TradingContext>,
    ) -> Self::Refined;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct GraphPrediction {
    pub signals: Vec<PredictedLongShortSignal>,
}

pub struct GraphPredictionWithTimeStampBuilder {
    pub graph_response: GraphPrediction,
    pub timezone: Tz,
}

impl GraphPredictionWithTimeStampBuilder {
    pub fn new(graph_response: GraphPrediction, timezone: Tz) -> Self {
        GraphPredictionWithTimeStampBuilder {
            graph_response,
            timezone,
        }
    }

    pub fn build(
        self,
        model_name: &str,
        prompt_hash: &str,
        context: Option<TradingContext>,
    ) -> RefinedGraphPrediction {
        let model_name = model_name.to_owned();
        let prompt_hash = prompt_hash.to_owned();

        let now_utc = Utc::now();
        let now_local = now_utc.with_timezone(&self.timezone);
        let iso_local = now_local.to_rfc3339();

        println!("🔥 self.graph_response:{:#?}", self.graph_response.clone());

        let signals = self
            .graph_response
            .signals
            .into_iter()
            .map(LongShortSignal::new)
            .collect();

        let timestamp = now_utc.timestamp_millis();

        RefinedGraphPrediction {
            context,
            current_time: timestamp,
            current_datetime: iso_local,
            signals,
            model_name,
            prompt_hash,
        }
    }
}

impl Refinable for GraphPrediction {
    type Refined = RefinedGraphPrediction;
    fn refine(
        self,
        timezone: Tz,
        model_name: &str,
        prompt_hash: &str,
        context: Option<TradingContext>,
    ) -> Self::Refined {
        GraphPredictionWithTimeStampBuilder::new(self, timezone).build(
            model_name,
            prompt_hash,
            context,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum KlineValue {
    Int64(i64),
    UInt64(u64),
    String(String),
    Float64(f64),
    UInt32(u32),
}

impl KlineValue {
    pub fn to_f64(&self) -> anyhow::Result<f64> {
        match self {
            KlineValue::Int64(val) => Ok(*val as f64),
            KlineValue::UInt64(val) => Ok(*val as f64),
            KlineValue::String(s) => s.parse::<f64>().map_err(|e| e.into()),
            KlineValue::Float64(val) => Ok(*val),
            KlineValue::UInt32(val) => Ok(*val as f64),
        }
    }

    pub fn to_json_value(&self) -> anyhow::Result<JsonValue> {
        match self {
            KlineValue::Int64(val) => Ok(JsonValue::Number(JsonNumber::from(*val))),
            KlineValue::UInt64(val) => Ok(JsonValue::Number(JsonNumber::from(*val))),
            KlineValue::String(s) => {
                let float_val = s
                    .parse::<f64>()
                    .with_context(|| format!("Failed to parse string '{}' as f64", s))?;
                Ok(JsonValue::Number(
                    JsonNumber::from_f64(float_val).ok_or_else(|| {
                        anyhow::anyhow!("Failed to convert {} to JsonNumber", float_val)
                    })?,
                ))
            }
            KlineValue::Float64(val) => Ok(JsonValue::Number(
                JsonNumber::from_f64(*val)
                    .ok_or_else(|| anyhow::anyhow!("Failed to convert {} to JsonNumber", val))?,
            )),
            KlineValue::UInt32(val) => Ok(JsonValue::Number(JsonNumber::from(*val))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RefinedGraphPrediction {
    pub context: Option<TradingContext>,
    pub current_time: i64,
    pub current_datetime: String,
    pub signals: Vec<LongShortSignal>,
    // Stats
    pub model_name: String,
    pub prompt_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RefinedGraphPredictionResponse {
    pub current_time: i64,
    pub current_datetime: String,
    pub signals: Vec<LongShortSignal>,
    // Stats
    pub model_name: String,
    pub prompt_hash: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TradingPrediction {
    pub summary: PredictedSummary,
    pub signals: Vec<PredictedLongShortSignal>,
    pub positions: Option<Vec<PredictedLongShortPosition>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RefinedTradingPrediction {
    pub current_time: i64,
    pub current_datetime: String,
    pub current_price: Option<f64>, // Made optional since context is optional
    pub summary: PredictedSummary,
    pub signals: Vec<LongShortSignal>,
    pub positions: Option<Vec<LongShortPosition>>,
    // Stats
    pub model_name: String,
    pub prompt_hash: String,
}

pub struct TradingPredictionWithTimeStampBuilder {
    pub ai_response: TradingPrediction,
    pub timezone: Tz,
}

impl TradingPredictionWithTimeStampBuilder {
    pub fn new(ai_response: TradingPrediction, timezone: Tz) -> Self {
        TradingPredictionWithTimeStampBuilder {
            ai_response,
            timezone,
        }
    }

    pub fn build(
        self,
        model_name: &str,
        prompt_hash: &str,
        context: Option<TradingContext>,
    ) -> RefinedTradingPrediction {
        let model_name = model_name.to_owned();
        let prompt_hash = prompt_hash.to_owned();

        let now_utc = Utc::now();
        let now_local = now_utc.with_timezone(&self.timezone);
        let iso_local = now_local.to_rfc3339();

        let signals = self
            .ai_response
            .signals
            .into_iter()
            .map(LongShortSignal::new)
            .collect();

        let (current_price, positions) = match context {
            Some(ctx) => {
                let preps_positions = ctx.maybe_preps_positions.unwrap_or_default();
                let positions = if preps_positions.is_empty() {
                    None
                } else {
                    Some(
                        self.ai_response
                            .positions
                            .unwrap_or_default()
                            .iter()
                            .enumerate()
                            .filter_map(|(i, predicted_position)| {
                                preps_positions.get(i).map(|preps_position| {
                                    LongShortPosition::new(
                                        preps_position.clone(),
                                        predicted_position.clone(),
                                    )
                                })
                            })
                            .collect::<Vec<_>>(),
                    )
                };
                (Some(ctx.current_price), positions)
            }
            None => (None, None), // No context, so no price or positions
        };

        let timestamp = now_utc.timestamp_millis();

        RefinedTradingPrediction {
            current_time: timestamp,
            current_datetime: iso_local,
            current_price,
            summary: self.ai_response.summary,
            signals,
            positions,
            model_name,
            prompt_hash,
        }
    }
}

impl Refinable for TradingPrediction {
    type Refined = RefinedTradingPrediction;
    fn refine(
        self,
        timezone: Tz,
        model_name: &str,
        prompt_hash: &str,
        context: Option<TradingContext>,
    ) -> Self::Refined {
        TradingPredictionWithTimeStampBuilder::new(self, timezone).build(
            model_name,
            prompt_hash,
            context,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictedSummary {
    // pub upper_bound: f64,
    // pub lower_bound: f64,
    // pub technical_resistance_4h: f64,
    // pub technical_support_4h: f64,
    // #[serde(deserialize_with = "deserialize_vec_tuples")]
    // pub top_bids_price_amount: Vec<Vec<f64>>,
    // #[serde(deserialize_with = "deserialize_vec_tuples")]
    // pub top_asks_price_amount: Vec<Vec<f64>>,
    pub vibe: String,
    pub detail: String,
    pub suggestion: String,
}

#[allow(unused)]
fn deserialize_vec_tuples<'de, D>(deserializer: D) -> Result<Vec<Vec<f64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let tuples: Vec<(f64, f64)> = Deserialize::deserialize(deserializer)?;
    Ok(tuples.into_iter().map(|(a, b)| vec![a, b]).collect())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TradingContext {
    pub token_symbol: String,
    pub pair_symbol: String,
    pub timeframe: String,
    pub current_price: f64,
    pub maybe_preps_positions: Option<Vec<PerpsPosition>>,
    pub maybe_trading_predictions: Option<Vec<RefinedTradingPrediction>>,
    pub kline_intervals: Vec<String>,
    pub stoch_rsi_intervals: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictedLongShortSignal {
    pub pair_symbol: String,
    pub direction: String,
    pub entry_price: f64,
    pub target_price: f64,
    pub entry_time: i64,
    pub target_time: i64,
    pub stop_loss: f64,
    pub rationale: String,
    pub confidence: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LongShortSignal {
    // Predicted
    #[serde(flatten)]
    pub predicted: PredictedLongShortSignal,
    // UI
    pub entry_time_local: String,
    pub target_time_local: String,
}

impl LongShortSignal {
    pub fn new(predicted: PredictedLongShortSignal) -> Self {
        // Convert target_time to Tokyo timezone
        let target_time_local = DateTime::from_timestamp(predicted.target_time / 1000, 0)
            .map(|utc_datetime| {
                let tokyo_datetime: DateTime<Tz> = utc_datetime.with_timezone(&Tokyo);
                tokyo_datetime.to_rfc3339()
            })
            .unwrap_or_else(|| {
                eprintln!("Failed to parse target_time: {}", predicted.target_time);
                String::new()
            });

        // Convert entry_time to Tokyo timezone
        let entry_time_local = DateTime::from_timestamp(predicted.entry_time / 1000, 0)
            .map(|utc_datetime| {
                let tokyo_datetime: DateTime<Tz> = utc_datetime.with_timezone(&Tokyo);
                tokyo_datetime.to_rfc3339()
            })
            .unwrap_or_else(|| {
                eprintln!("Failed to parse entry_time: {}", predicted.entry_time);
                String::new()
            });

        LongShortSignal {
            predicted,
            entry_time_local,
            target_time_local,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PredictedLongShortPosition {
    pub suggested_target_price: f64,
    pub suggested_stop_loss: f64,
    pub suggestion: String,
    pub rationale: String,
    pub confidence: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LongShortPosition {
    // Opened Position
    pub side: Side,
    pub token_symbol: String,
    pub entry_price: f64,
    pub leverage: f64,
    pub liquidation_price: f64,
    pub pnl_after_fees_usd: f64,
    pub value: f64,
    pub target_price: Option<f64>,
    pub stop_loss: Option<f64>,
    // Predicted
    pub suggested_target_price: f64,
    pub suggested_stop_loss: f64,
    pub suggestion: String,
    pub rationale: String,
    pub confidence: f64,
}

impl LongShortPosition {
    pub fn new(perps_position: PerpsPosition, predicted: PredictedLongShortPosition) -> Self {
        let token_symbol = get_by_address(&perps_position.market_mint)
            .expect("Not support token pair")
            .symbol
            .to_string();

        LongShortPosition {
            // Predicted
            suggested_target_price: predicted.suggested_target_price,
            suggested_stop_loss: predicted.suggested_stop_loss,
            suggestion: predicted.suggestion,
            rationale: predicted.rationale,
            confidence: predicted.confidence,
            // Opened Position
            side: perps_position.side,
            token_symbol,
            entry_price: perps_position.entry_price,
            leverage: perps_position.leverage,
            liquidation_price: perps_position.liquidation_price,
            pnl_after_fees_usd: perps_position.pnl_after_fees_usd,
            value: perps_position.value,
            target_price: perps_position.target_price,
            stop_loss: perps_position.stop_loss,
        }
    }
}
