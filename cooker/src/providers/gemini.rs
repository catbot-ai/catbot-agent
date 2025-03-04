use anyhow::{anyhow, Result};

use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use strum::AsRefStr;
use strum::EnumString;

use super::cleaner::try_parse_json_with_trailing_comma_removal;

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

            let parsed_output: T = try_parse_json_with_trailing_comma_removal(&output_string)
                .map_err(|error| {
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

#[derive(Serialize, Default, Clone)]
pub struct PriceHistory {
    pub price_history_1m: Option<String>, // 1-minute candlestick data (optional)
    pub price_history_5m: Option<String>, // 5-minute candlestick data (optional)
    pub price_history_1h: Option<String>, // 1-hour candlestick data (optional)
    pub price_history_4h: Option<String>, // 4-hour candlestick data (optional)
    pub price_history_1d: Option<String>, // 1-day candlestick data (optional)
}

impl PriceHistory {
    /// Generates a formatted string for price history data, including only the timeframes that are present.
    pub fn to_formatted_string(&self) -> String {
        let mut price_history_string = String::new();

        macro_rules! push_history_if_some {
            ($field:ident, $timeframe:expr) => {
                if let Some(data) = &self.$field {
                    price_history_string.push_str(&format!(
                        "**Price History ({} timeframe):**\n{}\n",
                        $timeframe, data
                    ));
                }
            };
        }

        // Use the macro to reduce repetition
        push_history_if_some!(price_history_1m, "1m");
        push_history_if_some!(price_history_5m, "5m");
        push_history_if_some!(price_history_1h, "1h");
        push_history_if_some!(price_history_4h, "4h");
        push_history_if_some!(price_history_1d, "1d");

        price_history_string
    }
}
