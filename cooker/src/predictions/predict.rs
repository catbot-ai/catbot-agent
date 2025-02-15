use utils::PredictionOutput;

use crate::providers::gemini::AiProvider;
use crate::{
    providers::gemini::{build_prompt_stage1, GeminiModel, GeminiProvider},
    sources::binance::{fetch_binance_kline_data, fetch_orderbook_depth},
};

use anyhow::Result;

pub async fn get_prediction(
    symbol: &str,
    provider: &GeminiProvider,
    model: &GeminiModel,
    limit: i32,
) -> Result<PredictionOutput> {
    println!("Fetching Kline data (5m)...");
    let kline_data_5m = fetch_binance_kline_data(symbol, "5m", limit).await?;
    let price_history_5m_json = serde_json::to_string_pretty(&kline_data_5m)?;

    println!("Fetching Kline data (1h)...");
    let kline_data_1h = fetch_binance_kline_data(symbol, "1h", limit).await?;
    let price_history_1h_json = serde_json::to_string_pretty(&kline_data_1h)?;

    println!("Fetching Kline data (4h)...");
    let kline_data_4h = fetch_binance_kline_data(symbol, "4h", limit).await?;
    let price_history_4h_json = serde_json::to_string_pretty(&kline_data_4h)?;

    // TODO: If we include this, it will over 1m token for 1000 point data.
    // println!("Fetching Kline data (1d)...");
    // let kline_data_1d = fetch_binance_kline_data(symbol, "1d", limit).await?;
    // let price_history_1d_json = serde_json::to_string_pretty(&kline_data_1d)?;

    println!("Fetching Order Book Depth...");
    let orderbook_data = fetch_orderbook_depth(symbol, limit).await?;
    let order_book_depth_json = serde_json::to_string_pretty(&orderbook_data)?;

    // --- Build Prompt for Gemini API ---
    println!("Building prompt for Gemini API...");
    let prompt = build_prompt_stage1(
        symbol,
        &price_history_5m_json,
        &price_history_1h_json,
        &price_history_4h_json,
        &order_book_depth_json,
        model,
    );

    // --- Call Gemini API ---
    println!("Calling Gemini API...");
    let gemini_response = provider
        .call_api::<PredictionOutput>(model, &prompt, None)
        .await?;

    Ok(gemini_response)
}
