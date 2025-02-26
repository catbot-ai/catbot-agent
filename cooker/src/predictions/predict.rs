use crate::{
    providers::gemini::{build_prompt, AiProvider, GeminiModel, GeminiProvider},
    sources::binance::{fetch_binance_kline_data, fetch_orderbook_depth},
};
use chrono_tz::Asia::Tokyo;
use common::{
    ConciseKline, PerpsPosition, PredictionOutput, PredictionOutputWithTimeStampBuilder,
    RefinedPredictionOutput,
};

use anyhow::Result;

pub async fn get_prediction(
    pair_symbol: &str,
    provider: &GeminiProvider,
    model: &GeminiModel,
    limit: i32,
    maybe_preps_positions: Option<Vec<PerpsPosition>>,
) -> Result<RefinedPredictionOutput> {
    // println!("Fetching Kline data (1s)...");
    let kline_data_1s = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1s", 1).await?;
    let current_price = kline_data_1s[0]
        .close_price
        .parse::<f64>()
        .expect("Invalid close price");
    // let price_history_1s_string = serde_json::to_string_pretty(&kline_data_1s)?;
    // println!("price_history_1s_string:{}", price_history_1s_string);

    // println!("Fetching Kline data (5m)...");
    let kline_data_5m = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "5m", limit).await?;
    let price_history_5m_string = serde_json::to_string_pretty(&kline_data_5m)?;
    // println!("price_history_5m_string:{}", price_history_5m_string);

    // println!("Fetching Kline data (1h)...");
    let kline_data_1h = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1h", limit).await?;
    let price_history_1h_string = serde_json::to_string_pretty(&kline_data_1h)?;

    // println!("Fetching Kline data (4h)...");
    let kline_data_4h = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "4h", limit).await?;
    let price_history_4h_string = serde_json::to_string_pretty(&kline_data_4h)?;

    // println!("Fetching Kline data (1d)...");
    let kline_data_1d = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1d", limit).await?;
    let price_history_1d_string = serde_json::to_string_pretty(&kline_data_1d)?;

    // println!("Fetching Order Book Depth...");
    let orderbook = fetch_orderbook_depth(pair_symbol, limit).await?;
    // let order_book_depth_string = serde_json::to_string_pretty(&orderbook)?;

    // --- Build Prompt for Gemini API ---
    println!("Building prompt for Gemini API...");
    let prompt = build_prompt(
        model,
        100f64,
        pair_symbol,
        current_price,
        &price_history_5m_string,
        &price_history_1h_string,
        &price_history_4h_string,
        &price_history_1d_string,
        orderbook,
        maybe_preps_positions,
    );

    println!("{prompt:?}");

    // --- Call Gemini API ---
    println!("Calling Gemini API...");
    let gemini_response = provider
        .call_api::<PredictionOutput>(model, &prompt, None)
        .await?;

    let prediction_output_with_timestamp =
        PredictionOutputWithTimeStampBuilder::new(gemini_response, Tokyo).build();

    Ok(prediction_output_with_timestamp)
}
