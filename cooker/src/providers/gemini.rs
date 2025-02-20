use anyhow::{anyhow, Result};

use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use strum::AsRefStr;
use strum::EnumString;

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
    pub index: i32,
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

#[derive(Debug, EnumString, AsRefStr, PartialEq, Eq)] // Use strum_macros
pub enum GeminiModel {
    #[strum(serialize = "gemini-2.0-flash-lite-preview-02-05")]
    FlashLitePreview,
    #[strum(serialize = "gemini-2.0-flash-thinking-exp-01-21")]
    FlashThinkingExp,
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
#[allow(clippy::too_many_arguments, unused)]
pub fn build_prompt(
    model: &GeminiModel,
    fund: f64,
    pair_symbol: &str,
    current_price: f64,
    price_history_5m: &str,
    price_history_1h: &str,
    price_history_4h: &str,
    price_history_1d: &str,
    order_amount_bids_csv: &str,
    order_amount_asks_csv: &str,
) -> String {
    let current_datetime = Utc::now();
    let symbol = pair_symbol
        .split("USDT")
        .next()
        .expect("Expect USDT as a suffix");

    let schema_instruction = format!(
        r#"**Instructions:**

- Do technical analysis on all history prices.
- Predict actionable trading signals based on the provided technical, order book and sentiment analysis for vary timeframe 4h, 6h, 12h.
- Concentrate on spike price that regularly occurred at the nearly same time for target_datetime.

**JSON Output:**
```json
{{
    "summary": {{
        "title": "string", // Short summary (less than 128 characters). E.g., "{symbol} Long Opportunity" or "{symbol} Neutral Market"
        "price": "number", // Current {symbol} price (precise decimals).
        "upper_bound": "number", // Current {symbol} upper bound (strongest resistance price).
        "lower_bound": "number", // Current {symbol} lower bound (strongest support price).
        "top_3_resistances": "[number]", // Top 3 ask prices with highest cumulative volume (highest to lowest volume).
        "top_3_supports": "[number]", // Top 3 bid prices with highest cumulative volume (highest to lowest volume).
        "technical_resistance_4h": "number", // Possible highest price in 4h timeframe.
        "technical_support_4h": "number", // Possible lowest price in 4h timeframe.
        "detail": "string", // Trade analysis summary (less than 255 characters). Include reasons for sentiment and signal generation or lack thereof. Mention any discrepancies.
        "suggestion": "string", // Suggested action. E.g., "Consider Long {symbol} if price holds above 173" or "Neutral. Observe price action." or "Consider Short {symbol} below 174."
        "vibe": "string" // Market sentiment with confidence percentage. E.g., "Bullish 65%", "Bearish 70%", "Neutral 80%".
    }},
    "long_signals": [
    {{
        "symbol": "{symbol}",
        "amount": "number",         // Calculated trade amount in {symbol} based on fund and entry price.
        "current_price": "number",  // Current {symbol} price in USD.
        "entry_price": "number",    // Suggested entry price for long position in USD.
        "target_price": "number",   // Target price for long position in USD.
        "stop_loss": "number",      // Stop loss price for long position in USD.
        "timeframe": "string",      // 1h, 4h, 6h, 12h, 1d, ...
        "target_datetime": "string",// Estimated target datetime in ISO format to reach target_price from {current_datetime}.
        "rationale": "string"       // Explanation for the long signal, referencing support, sentiment, etc.
    }}],
    "short_signals": [
    {{
        "symbol": "{symbol}",
        "amount": "number",         // Calculated trade amount in {symbol} based on fund and entry price.
        "current_price": "number",  // Current {symbol} price in USD.
        "entry_price": "number",    // Suggested entry price for short position in USD.
        "target_price": "number",   // Target price for short position in USD.
        "stop_loss": "number",      // Stop loss price for short position in USD.
        "timeframe": "string",      // 1h, 4h, 6h, 12h, 1d, ...
        "target_datetime": "string",// Estimated target datetime in ISO format to reach target_price from {current_datetime}.
        "rationale": "string"       // Explanation for the short signal, referencing resistance, sentiment, etc.
    }}]
}}

Be concise, Think step by step especially top_3_resistances and top_3_supports.
"#
    );

    format!(
        r#"Analyze the {symbol} market for potential price movement in the next 4 hours based on the following data.
Pay close attention to the *volume* of bids and asks when determining support and resistance.:

**Current Price:**
{current_price}

**Asks:**
{order_amount_asks_csv}

**Bids:**
{order_amount_bids_csv}

**Price History (1h timeframe):**
{price_history_1h}

**Price History (4h timeframe):**
{price_history_4h}

{schema_instruction}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        providers::gemini::GeminiModel,
        sources::binance::{fetch_binance_kline_data, fetch_orderbook_depth},
        transforms::numbers::{
            group_by_fractional_part, to_csv, top_n_support_resistance, FractionalPart,
        },
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

        let price_history_5m = "[]"; // Empty price history
        let price_history_1h = "[]";
        let price_history_4h = "[]";
        let price_history_1d = "[]";

        // let orderbook_json = r#"{"lastUpdateId":18560646066,
        // "bids":[["170.02000000","204.47900000"],["170.01000000","150.14900000"],["170.00000000","86.51000000"],["169.99000000","104.08900000"],["169.98000000","168.26600000"],["169.97000000","102.02100000"],["169.96000000","189.04000000"],["169.95000000","190.76100000"],["168.94000000","308.73800000"],["167.93000000","224.72800000"]],
        // "asks":[["170.03000000","12.03800000"],["170.04000000","3.84100000"],["170.05000000","34.67200000"],["170.06000000","90.68600000"],["170.07000000","200.38200000"],["170.08000000","98.31900000"],["170.09000000","102.28700000"],["170.10000000","196.39600000"],["171.11000000","191.37100000"],["172.12000000","169.14700000"]]}"#;
        // let orderbook: OrderBook = serde_json::from_str(orderbook_json).unwrap();
        // let (grouped_bids, grouped_asks) =
        //     group_by_fractional_part(&orderbook, FractionalPart::OneTenth);

        // let (_, top_bids) = top_n_support_resistance(&grouped_bids, 10);
        // let (top_asks, _) = top_n_support_resistance(&grouped_asks, 10);

        // let order_amount_bids_csv = to_csv(&top_bids);
        // let order_amount_asks_csv = to_csv(&top_asks);

        let orderbook = fetch_orderbook_depth("SOLUSDT", 1000).await.unwrap();
        // let order_book_depth_string = serde_json::to_string_pretty(&orderbook)?;

        let (grouped_bids, grouped_asks) =
            group_by_fractional_part(&orderbook, FractionalPart::One);

        let (_, top_bids) = top_n_support_resistance(&grouped_bids, 10);
        let (top_asks, _) = top_n_support_resistance(&grouped_asks, 10);

        let order_amount_bids_csv = to_csv(&top_bids);
        let order_amount_asks_csv = to_csv(&top_asks);

        let model = GeminiModel::FlashLitePreview; // Choose a model

        let prompt = build_prompt(
            &model,
            3f64,
            pair_symbol,
            current_price,
            price_history_5m,
            price_history_1h,
            price_history_4h,
            price_history_1d,
            &order_amount_bids_csv,
            &order_amount_asks_csv,
        );

        println!("\n--- Prompt Output for Empty Price History ---");
        println!("{}", prompt); // Print the prompt for inspection

        // You can add assertions here to check if the prompt is structured as expected
        // For example, you might want to check if certain keywords or data placeholders are present in the prompt string.

        Ok(())
    }
}
