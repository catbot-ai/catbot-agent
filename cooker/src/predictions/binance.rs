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
    // --- Define Required Data Specs ---
    let required_kline_intervals = [
        // Include intervals relevant for the desired analysis context
        "5m:864", "15m:672", "1h:168", "4h:84", "1d:100",
    ];

    // Include RSI or other indicators if desired in the report
    let stoch_rsi_intervals = ["1h:168", "4h:84"];

    // --- Fetch Data and Build Report String using Builder ---
    println!("Fetching historical data and building report string...");
    let builder = PriceHistoryBuilder::new(&context.pair_symbol, 100)
        .with_klines(&required_kline_intervals)
        .with_stoch_rsi(&stoch_rsi_intervals); // Add RSI to the report

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
        1000f64, // Example budget
    );

    println!("Prompt generated successfully.");
    // println!("{prompt:?}");
    Ok(prompt)
}
