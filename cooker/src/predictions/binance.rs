use super::prediction_types::PredictionType;
use crate::providers::{gemini::GeminiModel, prompter::build_prompt};
use anyhow::Context;
use common::{
    binance::fetch_orderbook_depth_usdt,
    transforms::csv::PriceHistoryBuilder, // Keep builder
    TradingContext,
};
// Removed: Kline, klines_to_csv, HashMap

pub async fn get_binance_prompt(
    prediction_type: &PredictionType,
    model: &GeminiModel,
    context: TradingContext,
    orderbook_limit: i32,
) -> anyhow::Result<String> {
    // --- Fetch Data and Build Report String using Builder ---
    println!("Fetching historical data and building report string...");
    let builder = PriceHistoryBuilder::new(&context.pair_symbol, 100)
        .with_klines(
            context
                .kline_intervals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .with_stoch_rsi(
            context
                .stoch_rsi_intervals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .with_latest_bb_ma(
            context
                .latest_bb_ma_intervals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
        );

    // Get the full report string from the builder
    let historical_data_content: String = builder
        .build()
        .await
        .context("Failed to build historical data report string using builder")?;

    // --- Fetch Orderbook ---
    println!("Fetching order book data...");
    let orderbook = fetch_orderbook_depth_usdt(&context.pair_symbol, orderbook_limit)
        .await
        .context("Failed to fetch orderbook depth")?;

    // --- Build Prompt ---
    println!("Building prompt for Gemini API...");
    // Pass the processed report string directly
    let prompt = build_prompt(
        prediction_type,
        model,
        context,
        historical_data_content, // Pass the generated content string
        orderbook,
    );

    println!("Prompt generated successfully.");
    // println!("{prompt:?}");
    Ok(prompt)
}
