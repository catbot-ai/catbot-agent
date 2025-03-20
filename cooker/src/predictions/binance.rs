use super::prediction_types::PredictionType;
use crate::providers::{core::PriceHistory, gemini::GeminiModel, prompter::build_prompt};
use common::{
    binance::{fetch_binance_kline_csv, fetch_orderbook_depth},
    TradingContext,
};

pub async fn get_binance_prompt(
    prediction_type: &PredictionType,
    model: &GeminiModel,
    context: TradingContext,
    orderbook_limit: i32,
) -> anyhow::Result<String> {
    // Fetch 1m kline data: 500 candles = ~8.3 hours
    let kline_data_1m = fetch_binance_kline_csv(&context.pair_symbol, "1m", 500 * 3 * 2).await?;

    // Fetch 5m kline data: 288 candles = 24h for 1-day short-term analysis
    let kline_data_5m = fetch_binance_kline_csv(&context.pair_symbol, "5m", 288 * 3).await?;

    // Fetch 1h kline data: 168 candles = 7d for 1h signal context
    let kline_data_1h = fetch_binance_kline_csv(&context.pair_symbol, "1h", 168).await?;

    // Fetch 4h kline data: 84 candles = 14d for 4h signals
    let kline_data_4h = fetch_binance_kline_csv(&context.pair_symbol, "4h", 84).await?;

    // Fetch 1d kline data: 100 candles = ~3m for long-term context
    let kline_data_1d = fetch_binance_kline_csv(&context.pair_symbol, "1d", 100).await?;

    let price_history = PriceHistory {
        price_history_1m: Some(kline_data_1m),
        price_history_5m: Some(kline_data_5m),
        price_history_1h: Some(kline_data_1h),
        price_history_4h: Some(kline_data_4h),
        price_history_1d: Some(kline_data_1d),
    };

    let orderbook = fetch_orderbook_depth(&context.pair_symbol, orderbook_limit).await?;

    // --- Build Prompt for Gemini API ---
    println!("Building prompt for Gemini API...");
    let prompt = build_prompt(
        prediction_type,
        model,
        context,
        Some(price_history),
        orderbook,
        1000f64,
    );

    println!("{prompt:?}");
    Ok(prompt)
}
