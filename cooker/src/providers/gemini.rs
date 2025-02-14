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
            let raw_response: GeminiResponse = response.json().await?;
            let output_string = raw_response
                .candidates
                .first()
                .and_then(|candidate| candidate.content.parts.first())
                .map(|part| part.text.clone())
                .ok_or_else(|| anyhow!("No text output found in Gemini response"))?;
            let parsed_output: T = serde_json::from_str(&output_string)?;

            Ok(parsed_output)
        } else {
            Err(anyhow!(
                "Gemini API request failed: {:?}",
                response.status()
            ))
        }
    }
}

// --- Prompt Building Function ---
#[allow(clippy::too_many_arguments, unused)]
pub fn build_prompt_stage1(
    symbol: &str,
    price_history_5m: &str,
    price_history_1h: &str,
    price_history_4h: &str,
    price_history_1d: &str,
    order_book_depth: &str,
    model: &GeminiModel,
) -> String {
    let schema_instruction = format!(
        r#"**IMPORTANT:** Format the output strictly as a JSON object, and ensure it adheres to the following JSON structure:

        ```json
        {{
          "summary": {{
            "title": "string",  // Some short word for notifications.
            "detail": "string", // Summary detail about technical analysis.
            "vibe": "string"    // Optional, e.g., "bullish", "bearish", "neutral"
          }},
          "buy_signals": [
            {{
              "price": number,
              "amount_usd": number,
              "amount_sol": number,
              "pair": "{symbol}"
            }}
          ],
          "sell_signals": [
            {{
              "price": number,
              "amount_usd": number,
              "amount_sol": number,
              "pair": "{symbol}"
            }}
          ],
          "price_prediction_graph": [
            {{
                "minute": number, 
                "price": number
            }}
          ]
        }}
        ```
        Ensure all keys are snake_case.
"#
    );

    format!(
        r#"Analyze the {} market for potential price movement in the next hour based on the following data:

        **Price History (5m timeframe):**
        {}

        **Price History (1h timeframe):**
        {}

        **Price History (4h timeframe):**
        {}

        **Price History (1d timeframe):**
        {}

        **Order Book Depth:**
        {}

        Perform technical analysis considering price trends, volatility, and order book depth.
        Identify key support and resistance levels.
        Based on a hypothetical $100 fund, suggest specific buy and sell signals (price, Token amount, USD amount) for {}. Include pair information in each signal.
        Provide a price prediction graph with price points every 5 minutes for the next hour.
        {}
        "#,
        symbol,
        price_history_5m,
        price_history_1h,
        price_history_4h,
        price_history_1d,
        order_book_depth,
        symbol,
        schema_instruction, // Conditionally include schema instruction
    )
}
