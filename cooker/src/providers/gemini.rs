use anyhow::{anyhow, Result};

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
pub fn build_prompt(
    symbol_with_usdt: &str,
    price_history_1s: &str,
    price_history_5m: &str,
    price_history_1h: &str,
    price_history_4h: &str,
    price_history_1d: &str,
    order_book_depth: &str,
    model: &GeminiModel,
) -> String {
    let symbol = symbol_with_usdt
        .split("USDT")
        .next()
        .expect("Expect USDT as a suffix");

    let fund = format!("1 {}", symbol);
    let schema_instruction = format!(
        r#"**IMPORTANT:** Format the output strictly as a valid JSON object, and ensure it adheres to the following JSON structure:

```json
{{
  "summary": {{
    "title": "string",         // Short summary less than 128 characters. e.g. "{symbol} long opportunity"
    "current_price": "number", // Current {symbol} price, precise decimals number in USD.
    "upper_bound": "number",   // Current {symbol} upper bound.
    "lower_bound": "number",   // Current {symbol} lower bound.
    "first_resistance": "number"  // Current {symbol} first significant amount of resistance.
    "first_support": "number"     // Current {symbol} first significant amount of support.
    "second_resistance": "number" // Current {symbol} second significant amount of resistance.
    "second_support": "number"    // Current {symbol} second significant amount of support.
    "detail": "string",       // Prediction trade analysis summary less than 255 characters.
    "suggestion": "string",   // Suggest action title e.g. "Consider long {symbol} in next 5 minutes" in ja
    "vibe": "string"          // Bear/Bull/Natural prediction with percent e.g. "Bull 100% in next hour".
  }},
  "long_signals": [
    {{
      "symbol": "{symbol}",
      "amount": "number",         // Calculate based on the {fund} fund and entry price, precise decimals as possible.
      "current_price": "number",  // Current {symbol} price, precise decimals number in USD.
      "entry_price": "number",    // Precise decimals number in USD.
      "target_price": "number",   // Precise decimals number in USD.
      "stop_loss": "number",      // Precise decimals number in USD.
      "timeframe": "string",      // Indicated expected time frame e.g. 5m, 15m, 1h, 4h, 1d, ...
      "rationale": "string"
    }}
  ],
  "short_signals": [
    {{
      "symbol": "{symbol}",
      "amount": "number",         // Calculate based on the {fund} fund and entry price, precise decimals as possible.
      "entry_price": "number",    // Precise decimals number in USD.
      "target_price": "number",   // Precise decimals number in USD.
      "stop_loss": "number",      // Precise decimals number in USD.
      "timeframe": "string",      // Indicated expected time frame e.g. 5m, 15m, 1h, 4h, 1d, ...
      "rationale": "string"
    }}
  ]
}}
```
Ensure all keys are snake_case. Numbers should be at least 3 decimals. Provide specific rationale, profit targets, and stop-loss levels. 
The long_signals and short_signals arrays should contain signals appropriate for their respective positions.  

Be concise and focus on profitable trades while managing the {fund} fund.
Consider $10 fees, especially for short positions (e.g., funding rates for perpetual contracts).
"#
    );

    format!(
        r#"Analyze the {symbol} market for potential price movement in the next 4 hours (240 minutes) based on the following data:

**Current Price:**
{price_history_1s}

**Price History (5m timeframe):**
{price_history_5m}

**Price History (1h timeframe):**
{price_history_1h}

**Price History (4h timeframe):**
{price_history_4h}

**Price History (1d timeframe):**
{price_history_1d}

**Order Book Depth:**
{order_book_depth}

Perform a comprehensive technical analysis for {symbol_with_usdt}, considering: Trend Analysis, Volatility, Support and Resistance, Order Book Analysis.
Based on a hypothetical {fund} fund, suggest 2-5 high-probability signals, separated into long_signals and short_signals.
e.g. for 1 {symbol} we will use 0.5 {symbol} for long and 0.5 {symbol} for short which mean we can long or short 0.1 {symbol} amount for each invest.

Do not suggest long or short if the profit will be less than $5.
Do suggest signals for vary timeframe that matched current(usually too small to have profit) and next support/resistant (usually to wide but profitable).
Also consider upper_bound and lower_bound (especially the one that match support/resistant) as a price target long and short signals.

Prioritize signals with timeframes of 1h, 4h, and potentially 1d, in addition to shorter-term 5m signals,
to capture both intraday and potential swing trading opportunities.

Be concise and focus on profitable trades while carefully managing the {fund} fund.
Consider fees, especially funding rates for short positions in perpetual contracts.

{schema_instruction}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::gemini::GeminiModel;
    use anyhow::Result;

    #[test]
    fn test_build_prompt_stage1_empty_price_history() -> Result<()> {
        let symbol_with_usdt = "SOLUSDT";
        let price_history_1s = r#"[
  {
    "open_time": 1739773540000,
    "open_price": "184.58",
    "high_price": "184.58",
    "low_price": "184.56",
    "close_price": "184.56",
    "volume": "8.911"
  }
]"#;
        let price_history_5m = "[]"; // Empty price history
        let price_history_1h = "[]";
        let price_history_4h = "[]";
        let price_history_1d = "[]";
        let order_book_depth = "{}"; // Empty order book

        println!("price_history_1s:{:#?}", price_history_1s);

        let model = GeminiModel::FlashLitePreview; // Choose a model

        let prompt = build_prompt(
            symbol_with_usdt,
            price_history_1s,
            price_history_5m,
            price_history_1h,
            price_history_4h,
            price_history_1d,
            order_book_depth,
            &model,
        );

        println!("\n--- Prompt Output for Empty Price History ---");
        println!("{}", prompt); // Print the prompt for inspection

        // You can add assertions here to check if the prompt is structured as expected
        // For example, you might want to check if certain keywords or data placeholders are present in the prompt string.

        Ok(())
    }
}
