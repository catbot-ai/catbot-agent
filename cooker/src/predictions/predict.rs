use common::{ClosePriceKline, PredictionOutput, PredictionOutputWithTimeStamp};

use crate::providers::gemini::AiProvider;
use crate::transforms::numbers::{
    group_by_fractional_part, to_csv, top_n_support_resistance, FractionalPart,
};
use crate::{
    providers::gemini::{build_prompt, GeminiModel, GeminiProvider},
    sources::binance::{fetch_binance_kline_data, fetch_orderbook_depth},
};

use anyhow::Result;

pub async fn get_prediction(
    pair_symbol: &str,
    provider: &GeminiProvider,
    model: &GeminiModel,
    limit: i32,
) -> Result<PredictionOutputWithTimeStamp> {
    // println!("Fetching Kline data (1s)...");
    let kline_data_1s = fetch_binance_kline_data::<ClosePriceKline>(pair_symbol, "1s", 1).await?;
    let current_price = kline_data_1s[0]
        .close_price
        .parse::<f64>()
        .expect("Invalid close price");
    // let price_history_1s_string = serde_json::to_string_pretty(&kline_data_1s)?;
    // println!("price_history_1s_string:{}", price_history_1s_string);

    // println!("Fetching Kline data (5m)...");
    let kline_data_5m =
        fetch_binance_kline_data::<ClosePriceKline>(pair_symbol, "5m", limit).await?;
    let price_history_5m_string = serde_json::to_string_pretty(&kline_data_5m)?;
    // println!("price_history_5m_string:{}", price_history_5m_string);

    // println!("Fetching Kline data (1h)...");
    let kline_data_1h =
        fetch_binance_kline_data::<ClosePriceKline>(pair_symbol, "1h", limit).await?;
    let price_history_1h_string = serde_json::to_string_pretty(&kline_data_1h)?;

    // println!("Fetching Kline data (4h)...");
    let kline_data_4h =
        fetch_binance_kline_data::<ClosePriceKline>(pair_symbol, "4h", limit).await?;
    let price_history_4h_string = serde_json::to_string_pretty(&kline_data_4h)?;

    // println!("Fetching Kline data (1d)...");
    let kline_data_1d =
        fetch_binance_kline_data::<ClosePriceKline>(pair_symbol, "1d", limit).await?;
    let price_history_1d_string = serde_json::to_string_pretty(&kline_data_1d)?;

    // println!("Fetching Order Book Depth...");
    let orderbook = fetch_orderbook_depth(pair_symbol, limit).await?;
    // let order_book_depth_string = serde_json::to_string_pretty(&orderbook)?;

    let (grouped_bids, grouped_asks) = group_by_fractional_part(&orderbook, FractionalPart::One);

    let (top_asks, _) = top_n_support_resistance(&grouped_asks, 10);
    let (_, top_bids) = top_n_support_resistance(&grouped_bids, 10);

    let order_amount_asks_csv = to_csv(&top_asks);
    let order_amount_bids_csv = to_csv(&top_bids);

    // println!("{group_by_fractional_part_csv_string:?}");

    // --- Build Prompt for Gemini API ---
    println!("Building prompt for Gemini API...");
    let prompt = build_prompt(
        model,
        3f64,
        pair_symbol,
        current_price,
        &price_history_5m_string,
        &price_history_1h_string,
        &price_history_4h_string,
        &price_history_1d_string,
        &order_amount_bids_csv,
        &order_amount_asks_csv,
    );

    println!("{prompt:?}");

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
            // price_prediction_graph_5m: gemini_response.price_prediction_graph_5m,
        };

    Ok(prediction_output_with_timestamp)
}
