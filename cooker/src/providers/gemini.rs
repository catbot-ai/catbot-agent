use anyhow::{anyhow, Result};

use chrono::Utc;
use common::OrderBook;
use common::PerpsPosition;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use strum::AsRefStr;
use strum::EnumString;

use crate::transforms::numbers::btree_map_to_csv;
use crate::transforms::numbers::group_by_fractional_part;
use crate::transforms::numbers::top_n_bids_asks;
use crate::transforms::numbers::FractionalPart;

// --- Gemini Model Enum ---

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: UsageMetadata,
    pub model_version: String,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Content,
    pub finish_reason: String,
    pub avg_logprobs: f64,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    pub parts: Vec<Part>,
    pub role: String,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub text: String,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: i32,
    pub total_token_count: i32,
}

#[derive(Default, Debug, EnumString, AsRefStr, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum GeminiModel {
    #[default]
    #[strum(serialize = "gemini-2.0-flash-lite")]
    Gemini2FlashLite,
    #[strum(serialize = "gemini-2.0-flash")]
    Gemini2Flash,
    #[strum(serialize = "gemini-2.0-flash-thinking-exp-01-21")]
    Gemini2FlashThinkingExp,
}

pub trait AiProvider {
    async fn call_api<T: serde::de::DeserializeOwned + Send>(
        &self,
        model: &GeminiModel,
        prompt: &str,
        maybe_response_schema: Option<&str>,
    ) -> Result<T>;
}

pub struct GeminiProvider {
    pub client: Arc<Client>,
    pub api_url: String,
    pub api_key: String,
}

impl GeminiProvider {
    pub fn new(api_url: &str, api_key: &str) -> Self {
        GeminiProvider {
            client: Arc::new(Client::new()),
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub fn new_v1beta(api_key: &str) -> Self {
        GeminiProvider::new(
            "https://generativelanguage.googleapis.com/v1beta/models/",
            api_key,
        )
    }
}

impl AiProvider for GeminiProvider {
    async fn call_api<T: serde::de::DeserializeOwned + Send>(
        &self,
        model: &GeminiModel,
        prompt: &str,
        maybe_response_schema: Option<&str>,
    ) -> Result<T> {
        let model_str = model.as_ref();
        let gemini_api_url = format!(
            "{}{}:generateContent?key={}",
            self.api_url, model_str, self.api_key
        );

        let payload_json = if let Some(response_schema) = maybe_response_schema {
            json!({
                "contents": [{
                  "parts":[
                    {"text": prompt}
                  ]
                }],
                "generationConfig": {
                    "response_mime_type": "application/json",
                    "response_schema": response_schema,
                }
            })
        } else {
            json!({
                "contents": [{
                  "parts":[
                    {"text": prompt}
                  ]
                }],
                "generationConfig": {
                    "response_mime_type": "application/json",
                }
            })
        };

        let response = self
            .client
            .post(gemini_api_url)
            .json(&payload_json)
            .send()
            .await?;

        if response.status().is_success() {
            let raw_text_response = response.text().await?;

            let raw_response: GeminiResponse =
                serde_json::from_str(&raw_text_response).map_err(|e| {
                    anyhow!("Failed to deserialize GeminiResponse from raw text: {}", e)
                })?;

            let output_string = raw_response
                .candidates
                .first()
                .and_then(|candidate| candidate.content.parts.first())
                .map(|part| part.text.clone())
                .ok_or_else(|| anyhow!("No text output found in Gemini response"))?;
            let parsed_output: T = serde_json::from_str(&output_string).map_err(|error| {
                anyhow!(
                    "Raw Gemini API Response: {}, error: {}",
                    &raw_text_response,
                    error
                )
            })?;

            Ok(parsed_output)
        } else {
            Err(anyhow!(
                "Gemini API request failed: {:?}",
                response.status()
            ))
        }
    }
}

#[allow(clippy::too_many_arguments, unused)]
pub fn build_prompt(
    model: &GeminiModel,
    fund_usd: f64,
    pair_symbol: &str,
    current_price: f64,
    price_history_5m: &str,
    price_history_1h: &str,
    price_history_4h: &str,
    price_history_1d: &str,
    orderbook: OrderBook,
    maybe_preps_positions: Option<Vec<PerpsPosition>>,
) -> String {
    let current_datetime = Utc::now();
    let current_timestamp = Utc::now().timestamp_millis();
    let symbol = pair_symbol.split("USDT").next().unwrap_or(pair_symbol);

    let (grouped_one_bids, grouped_one_asks) =
        group_by_fractional_part(&orderbook, FractionalPart::One);
    let (grouped_one_tenth_bids, grouped_one_tenth_asks) =
        group_by_fractional_part(&orderbook, FractionalPart::OneTenth);

    // Limit 10
    let top_bids_price_amount = top_n_bids_asks(&grouped_one_bids, 5, false);
    let top_asks_price_amount = top_n_bids_asks(&grouped_one_asks, 5, true);

    let grouped_bids_string = btree_map_to_csv(&grouped_one_bids);
    let grouped_asks_string = btree_map_to_csv(&grouped_one_asks);

    let min_profit = fund_usd * 0.025;

    let maybe_position_schema = if let Some(preps_positions) = maybe_preps_positions {
        let mut positions_string = String::from(
            r#",
"positions": ["#,
        );
        for preps_position in preps_positions.iter() {
            let side = preps_position.side.to_string();
            let symbol = preps_position.symbol.to_string();
            let entry_price = preps_position.entry_price;
            let leverage = preps_position.leverage;
            let liquidation_price = preps_position.liquidation_price;
            let pnl_after_fees_usd = preps_position.pnl_after_fees_usd;
            let value = preps_position.value;

            positions_string.push_str(&format!(r#"
{{
    "side": "{side}",
    "symbol": "{symbol}",
    "entry_price": {entry_price},
    "leverage": {leverage},
    "liquidation_price": {liquidation_price},
    "pnl_after_fees_usd": {pnl_after_fees_usd},
    "value": {value},
    "confidence": number, // 0.0-1.0
    "suggested_target_price": Option<number>, // Suggestion for new target_price (optional)
    "suggested_stop_loss": Option<number>, // Suggestion for new stop_loss (optional)
    "suggested_add_value": Option<number>  // Suggestion to add more value to position or not (optional)
}},
"#
            ));
        }
        // Remove latest ','
        positions_string.pop();
        positions_string.push_str("]\n");
        positions_string
    } else {
        "]\n".to_string()
    };

    let schema_instruction = format!(
        r#"**Instructions:**

- Perform technical analysis on price histories (5m, 1h, 4h, 1d) and order book volume.
- Generate trading signals with at least 2.5% profit potential from `entry_price` to `target_price`, ensuring a minimum 2.5% return on `fund_usd`. E.g., for `fund_usd` ${fund_usd}, profit at least $${min_profit}.
- Use 5m history for 1h signals (target_datetime within 1-2h) and 4h for 4h+ signals.
- Quantify bid/ask volume along with technical analysis in rationale and detail (e.g., "bids at 158 total 15438 SOL vs. asks at 160 total 17671 SOL").
- Identify recurring price spikes in history and align target_datetime accordingly.
- Match suggestion to signals; explain discrepancies if no signals.
- Take a look for each positions if has and suggest rebalance if need.

**JSON Output:**
```json
{{
    "summary": {{
        "title": "string", // E.g., "{symbol} Short-term Bearish"
        "price": {current_price},
        "upper_bound": number, // Highest top_3_resistance
        "lower_bound": number, // Lowest top_3_support
        "technical_resistance_4h": number, // From 4h analysis
        "technical_support_4h": number, // From 4h analysis
        "top_bids_price_amount": {top_bids_price_amount:?},
        "top_asks_price_amount": {top_asks_price_amount:?},
        "detail": "string", // <500 chars, include volume and momentum insights
        "suggestion": "string", // E.g., "Short {symbol} at 170.1 if volume confirms resistance"
        "vibe": "string" // E.g., "Bearish 65%", match signal confidence
    }},
    "signals": [{{
        "side": string, // long or shot
        "symbol": "{symbol}",
        "confidence": number, // Confidence about this signal: 0.0-1.0
        "current_price": {current_price},
        "entry_price": number,
        "target_price": number, // >2.5% above entry, beyond first resistance or support
        "stop_loss": number, // The value should less than profit.
        "timeframe": "string", // "1h" or "4h"
        "entry_datetime": "string", // ISO time prediction when to make a trade for this signal, Can be now or in the future date time.
        "target_datetime": "string", // ISO time prediction when to take profit.
        "rationale": "string" // E.g., "4h momentum up, bids outpace asks", "1h rejection at 170, high ask volume"
    }}]{maybe_position_schema}
}}
```
Be concise, Think step by step.
"#
    );

    format!(
        r#"Analyze {symbol} for price movement in the next 4 hours using:

## Input Data:

fund_usd={fund_usd}
current_datetime={current_datetime}
current_timestamp={current_timestamp}
current_price={current_price}

## Historical Data:

**Price History (1d timeframe):**
{price_history_1d}

**Price History (4h timeframe):**
{price_history_4h}

**Price History (1h timeframe):**
{price_history_1h}

**Price History (5m timeframe):**
{price_history_5m}

## Consolidated Data:

**Bids:**
{grouped_bids_string}

**Asks:**
{grouped_asks_string}

{schema_instruction}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        providers::gemini::GeminiModel,
        sources::binance::{fetch_binance_kline_data, fetch_orderbook_depth},
    };
    use anyhow::Result;
    use common::ConciseKline;

    #[tokio::test]
    async fn test_build_prompt_stage1_empty_price_history() -> Result<()> {
        let pair_symbol = "SOLUSDT";

        let kline_data_1s = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1s", 1).await?;
        let current_price = kline_data_1s[0]
            .close_price
            .parse::<f64>()
            .expect("Invalid close price");

        let price_history_5m = "[]";
        let price_history_1h = "[]";
        let price_history_4h = "[]";
        let price_history_1d = "[]";

        let orderbook = fetch_orderbook_depth("SOLUSDT", 100).await.unwrap();
        let model = GeminiModel::default();

        let prompt = build_prompt(
            &model,
            1000f64,
            pair_symbol,
            current_price,
            price_history_5m,
            price_history_1h,
            price_history_4h,
            price_history_1d,
            orderbook,
            None,
        );

        println!("{}", prompt);

        Ok(())
    }
}
