use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use strum::AsRefStr;
use strum::EnumString;

use super::cleaner::try_parse_json_with_trailing_comma_removal;
use super::core::AiProvider;

// --- Gemini Model Enum and Response Structs ---

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
#[serde(untagged)]
pub enum Part {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineDataContent,
    },
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineDataContent {
    mime_type: String,
    data: String, // Base64 encoded image data
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

#[derive(Serialize)]
pub struct ImageData {
    pub mime_type: String,
    pub data: String, // Base64 encoded image data
}

impl AiProvider for GeminiProvider {
    async fn call_api<T: serde::de::DeserializeOwned + Send>(
        &self,
        model: &GeminiModel,
        prompt: &str,
        maybe_response_schema: Option<&str>,
    ) -> Result<T> {
        self.call_api_with_images(model, prompt, vec![], maybe_response_schema)
            .await
    }
}

impl GeminiProvider {
    pub async fn call_api_with_images<T: serde::de::DeserializeOwned + Send>(
        &self,
        model: &GeminiModel,
        prompt: &str,
        images: Vec<ImageData>,
        maybe_response_schema: Option<&str>,
    ) -> Result<T> {
        let model_str = model.as_ref();
        let gemini_api_url = format!(
            "{}{}:generateContent?key={}",
            self.api_url, model_str, self.api_key
        );

        let mut parts = vec![Part::Text {
            text: prompt.to_string(),
        }];
        for image_data in images {
            parts.push(Part::InlineData {
                inline_data: InlineDataContent {
                    mime_type: image_data.mime_type,
                    data: image_data.data,
                },
            });
        }

        let payload_json = if let Some(response_schema) = maybe_response_schema {
            json!({
                "contents": [{
                    "parts": parts
                }],
                "generationConfig": {
                    "response_mime_type": "application/json",
                    "response_schema": response_schema,
                }
            })
        } else {
            json!({
                "contents": [{
                    "parts": parts
                }],
                "generationConfig": {
                    "response_mime_type": "application/json",
                }
            })
        };

        // Log the request payload for debugging
        println!("Request URL: {}", gemini_api_url);
        println!(
            "Request Payload: {}",
            serde_json::to_string_pretty(&payload_json)?
        );

        let response = self
            .client
            .post(&gemini_api_url)
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
                .and_then(|part| match part {
                    Part::Text { text } => Some(text.clone()),
                    _ => None,
                })
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
            let status = response.status();
            let headers = response.headers().clone();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());

            Err(anyhow!(
                "Gemini API request failed: Status: {}, Headers: {:?}, Body: {}",
                status,
                headers,
                error_body
            ))
        }
    }
}
