use super::cleaner::try_parse_json_with_trailing_comma_removal;
use super::core::AiProvider;
use anyhow::{anyhow, Result};
use json_schema::ToJsonSchema;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;
use strum::AsRefStr;
use strum::EnumString;

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

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum Part {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineDataContent,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCallContent,
    },
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InlineDataContent {
    mime_type: String,
    data: String, // Base64 encoded image data
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallContent {
    pub name: String,
    pub args: JsonValue,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: i32,
    pub total_token_count: i32,
}

#[derive(Default, Debug, EnumString, AsRefStr, PartialEq, Eq, Clone)]
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

// Unified builder for API calls
pub struct GeminiCallBuilder<'a> {
    provider: &'a GeminiProvider,
    model: &'a GeminiModel,
    prompt: String,
    images: Vec<ImageData>,
    response_schema: Option<String>,
    function_declarations: Vec<JsonValue>,
}

impl<'a> GeminiCallBuilder<'a> {
    pub fn new(provider: &'a GeminiProvider, model: &'a GeminiModel, prompt: String) -> Self {
        Self {
            provider,
            model,
            prompt,
            images: Vec::new(),
            response_schema: None,
            function_declarations: Vec::new(),
        }
    }

    pub fn with_images(mut self, images: Vec<ImageData>) -> Self {
        self.images = images;
        self
    }

    pub fn with_response_schema(mut self, schema: String) -> Self {
        self.response_schema = Some(schema);
        self
    }

    pub fn with_function_declarations<T: ToJsonSchema>(mut self, declarations: Vec<T>) -> Self {
        self.function_declarations = declarations
            .into_iter()
            .map(|_| T::to_json_schema())
            .collect();
        self
    }

    pub async fn run<T: serde::de::DeserializeOwned + Send>(self) -> Result<T> {
        let model_str = self.model.as_ref();
        let gemini_api_url = format!(
            "{}{}:generateContent?key={}",
            self.provider.api_url, model_str, self.provider.api_key
        );

        let mut parts = vec![Part::Text { text: self.prompt }];
        for image_data in self.images {
            parts.push(Part::InlineData {
                inline_data: InlineDataContent {
                    mime_type: image_data.mime_type,
                    data: image_data.data,
                },
            });
        }

        let mut payload_json = json!({
            "contents": [{"parts": parts}],
            "generationConfig": {"response_mime_type": "application/json"}
        });

        if let Some(response_schema) = self.response_schema {
            payload_json["generationConfig"]["response_schema"] = json!(response_schema);
        }

        if !self.function_declarations.is_empty() {
            payload_json["tools"] = json!([{"function_declarations": self.function_declarations}]);
        }

        println!("Request URL: {}", gemini_api_url);
        println!(
            "Request Payload: {}",
            serde_json::to_string_pretty(&payload_json)?
        );

        let response = self
            .provider
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

            let first_part = raw_response
                .candidates
                .first()
                .and_then(|candidate| candidate.content.parts.first())
                .ok_or_else(|| anyhow!("No content found in Gemini response"))?;

            match first_part {
                Part::Text { text } => {
                    let parsed_output: T = try_parse_json_with_trailing_comma_removal(text)
                        .map_err(|error| {
                            anyhow!(
                                "Raw Gemini API Response: {}, error: {}",
                                &raw_text_response,
                                error
                            )
                        })?;
                    Ok(parsed_output)
                }
                Part::FunctionCall { function_call } => {
                    let parsed_output: T = serde_json::from_value(json!(function_call))
                        .map_err(|e| anyhow!("Failed to deserialize function call: {}", e))?;
                    Ok(parsed_output)
                }
                _ => Err(anyhow!("Unexpected response part type")),
            }
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

    pub fn call_api<'a>(&'a self, model: &'a GeminiModel, prompt: String) -> GeminiCallBuilder<'a> {
        GeminiCallBuilder::new(self, model, prompt)
    }
}

impl AiProvider for GeminiProvider {
    async fn call_api<T: serde::de::DeserializeOwned + Send>(
        &self,
        model: &GeminiModel,
        prompt: &str,
        maybe_response_schema: Option<&str>,
    ) -> Result<T> {
        let mut builder = self.call_api(model, prompt.to_string());
        if let Some(schema) = maybe_response_schema {
            builder = builder.with_response_schema(schema.to_string());
        }
        builder.run().await
    }
}

#[derive(Serialize)]
pub struct ImageData {
    pub mime_type: String,
    pub data: String, // Base64 encoded image data
}
