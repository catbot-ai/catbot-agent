use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

// Define the function declaration for execute_trade_decision
pub fn get_trade_decision_function_declaration() -> JsonValue {
    json!({
        "name": "execute_trade_decision",
        "description": "Decide whether to execute a trade based on analysis of charts and signals",
        "parameters": {
            "type": "object",
            "properties": {
                "pair_symbol": {
                    "type": "string",
                    "description": "The trading pair symbol, e.g., SOL_USDT"
                },
                "should_trade": {
                    "type": "boolean",
                    "description": "Whether to execute the trade (true) or not (false)"
                },
                "rationale": {
                    "type": "string",
                    "description": "A brief explanation of the decision to trade or not"
                }
            },
            "required": [
                "pair_symbol",
                "should_trade",
                "rationale"
            ]
        }
    })
}

use crate::providers::gemini::{GeminiModel, GeminiProvider, ImageData};
use anyhow::{bail, Result};

// Struct to hold the trade decision result
#[derive(Debug, Serialize, Deserialize)]
pub struct TradeDecision {
    pub pair_symbol: String,
    pub should_trade: bool,
    pub rationale: String,
}

pub async fn analyze_and_decide_trade(
    provider: &GeminiProvider,
    model: &GeminiModel,
    prompt: &str,
    images: Vec<ImageData>,
) -> Result<TradeDecision> {
    // Construct the payload with the prompt, images, and function declaration
    let mut parts = vec![json!({ "text": prompt })];
    for image_data in images {
        parts.push(json!({
            "inlineData": {
                "mimeType": image_data.mime_type,
                "data": image_data.data
            }
        }));
    }

    let payload = json!({
        "contents": [{
            "parts": parts
        }],
        "tools": [{
            "function_declarations": [get_trade_decision_function_declaration()]
        }],
        "generationConfig": {
            "response_mime_type": "application/json"
        }
    });

    // Log the payload for debugging
    println!(
        "Request Payload: {}",
        serde_json::to_string_pretty(&payload)?
    );

    // Make the API call
    let model_str = model.as_ref();
    let gemini_api_url = format!(
        "{}{}:generateContent?key={}",
        provider.api_url, model_str, provider.api_key
    );

    let response = provider
        .client
        .post(&gemini_api_url)
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        let response_json: JsonValue = response.json().await?;

        // Log the response for debugging
        println!(
            "Response: {}",
            serde_json::to_string_pretty(&response_json)?
        );

        // Extract the function call from the response
        let function_call = response_json
            .get("candidates")
            .and_then(|candidates| candidates.get(0))
            .and_then(|candidate| candidate.get("content"))
            .and_then(|content| content.get("parts"))
            .and_then(|parts| parts.get(0))
            .and_then(|part| part.get("functionCall"))
            .ok_or_else(|| anyhow::anyhow!("No function call found in response"))?;

        let function_name = function_call
            .get("name")
            .and_then(|name| name.as_str())
            .ok_or_else(|| anyhow::anyhow!("Function name missing in response"))?;

        if function_name != "execute_trade_decision" {
            return Err(anyhow::anyhow!(
                "Unexpected function name: {}, expected execute_trade_decision",
                function_name
            ));
        }

        let args = function_call
            .get("args")
            .ok_or_else(|| anyhow::anyhow!("Function arguments missing in response"))?;

        let pair_symbol = args
            .get("pair_symbol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("pair_symbol missing in function arguments"))?
            .to_string();

        let should_trade = args
            .get("should_trade")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow::anyhow!("should_trade missing in function arguments"))?;

        let rationale = args
            .get("rationale")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("rationale missing in function arguments"))?
            .to_string();

        // Fake call to execute_trade_decision (placeholder)
        todo!("Implement execute_trade_decision with pair_symbol: {}, should_trade: {}, rationale: {}", pair_symbol, should_trade, rationale);

        // Return the decision
        Ok(TradeDecision {
            pair_symbol,
            should_trade,
            rationale,
        })
    } else {
        let status = response.status();
        let headers = response.headers().clone();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        Err(bail!(
            "Gemini API request failed: Status: {}, Headers: {:?}, Body: {}",
            status,
            headers,
            error_body
        ))
    }
}
