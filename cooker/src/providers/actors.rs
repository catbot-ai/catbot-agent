use crate::providers::gemini::{FunctionCallContent, GeminiModel, GeminiProvider, ImageData};
use anyhow::{anyhow, Result};
use json_schema_derive::ToJsonSchema;
use serde::{Deserialize, Serialize};

// TradeDecision struct
#[derive(Default, Debug, Serialize, Deserialize, ToJsonSchema)]
#[gemini(
    name = "execute_trade_decision",
    description = "Decide whether to execute a trade based on analysis of charts and signals"
)]
pub struct TradeDecision {
    #[gemini(description = "The trading pair symbol, e.g., SOL_USDT")]
    pub pair_symbol: String,
    #[gemini(description = "Whether to execute the trade (true) or not (false)")]
    pub should_trade: bool,
    #[gemini(description = "A brief explanation of the decision to trade or not")]
    pub rationale: String,
}

pub async fn analyze_and_decide_trade(
    provider: &GeminiProvider,
    model: &GeminiModel,
    prompt: &str,
    images: Option<Vec<ImageData>>,
) -> Result<TradeDecision> {
    let mut builder = provider
        .call_api(model, prompt.to_string())
        .with_function_declarations(vec![TradeDecision::default()]);

    if let Some(images) = images {
        builder = builder.with_images(images);
    }

    let function_call: FunctionCallContent = builder.run().await?;

    if function_call.name != "execute_trade_decision" {
        return Err(anyhow!(
            "Unexpected function name: {}, expected execute_trade_decision",
            function_call.name
        ));
    }

    let trade_decision: TradeDecision =
        serde_json::from_value(function_call.args).map_err(|e| {
            anyhow!(
                "Failed to deserialize function arguments into TradeDecision: {}",
                e
            )
        })?;

    // Placeholder for actual execution logic
    todo!(
        "Implement execute_trade_decision with pair_symbol: {}, should_trade: {}, rationale: {}",
        trade_decision.pair_symbol,
        trade_decision.should_trade,
        trade_decision.rationale
    );

    Ok(trade_decision)
}
