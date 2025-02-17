use utils::{ConciseKline, PredictionOutput, PredictionOutputWithTimeStamp};

use crate::providers::gemini::AiProvider;
use crate::{
    providers::gemini::{build_prompt, GeminiModel, GeminiProvider},
    sources::binance::{fetch_binance_kline_data, fetch_orderbook_depth},
};

use anyhow::Result;

pub async fn get_prediction(
    symbol: &str,
    provider: &GeminiProvider,
    model: &GeminiModel,
    limit: i32,
) -> Result<PredictionOutputWithTimeStamp> {
    println!("Fetching Kline data (1s)...");
    let kline_data_1s = fetch_binance_kline_data::<ConciseKline>(symbol, "1s", 1).await?;
    let price_history_1s_string = serde_json::to_string_pretty(&kline_data_1s)?;
    println!("price_history_1s_string:{}", price_history_1s_string);

    println!("Fetching Kline data (5m)...");
    let kline_data_5m = fetch_binance_kline_data::<ConciseKline>(symbol, "5m", limit).await?;
    let price_history_5m_string = serde_json::to_string_pretty(&kline_data_5m)?;
    println!("price_history_5m_string:{}", price_history_5m_string);

    println!("Fetching Kline data (1h)...");
    let kline_data_1h = fetch_binance_kline_data::<ConciseKline>(symbol, "1h", limit).await?;
    let price_history_1h_string = serde_json::to_string_pretty(&kline_data_1h)?;

    println!("Fetching Kline data (4h)...");
    let kline_data_4h = fetch_binance_kline_data::<ConciseKline>(symbol, "4h", limit).await?;
    let price_history_4h_string = serde_json::to_string_pretty(&kline_data_4h)?;

    println!("Fetching Kline data (1d)...");
    let kline_data_1d = fetch_binance_kline_data::<ConciseKline>(symbol, "1d", limit).await?;
    let price_history_1d_string = serde_json::to_string_pretty(&kline_data_1d)?;

    println!("Fetching Order Book Depth...");
    let orderbook_data = fetch_orderbook_depth(symbol, limit).await?;
    let order_book_depth_string = serde_json::to_string_pretty(&orderbook_data)?;

    // --- Build Prompt for Gemini API ---
    println!("Building prompt for Gemini API...");
    let prompt = build_prompt(
        symbol,
        &price_history_1s_string,
        &price_history_5m_string,
        &price_history_1h_string,
        &price_history_4h_string,
        &price_history_1d_string,
        &order_book_depth_string,
        model,
    );

    // --- Call Gemini API ---
    println!("Calling Gemini API...");
    let gemini_response = provider
        .call_api::<PredictionOutput>(model, &prompt, None)
        .await?;

    let prediction_output_with_timestamp: PredictionOutputWithTimeStamp =
        PredictionOutputWithTimeStamp {
            timestamp: chrono::Utc::now().timestamp_millis(),
            summary: gemini_response.summary,
            long_signals: gemini_response.long_signals,
            short_signals: gemini_response.short_signals,
            price_prediction_graph_5m: gemini_response.price_prediction_graph_5m,
        };

    Ok(prediction_output_with_timestamp)
}
