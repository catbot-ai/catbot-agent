use super::prediction_types::PredictionType;
use crate::providers::{core::PriceHistory, gemini::GeminiModel, prompter::build_prompt};
use common::{
    binance::{fetch_binance_kline_data, fetch_orderbook_depth},
    ConciseKline,
};
use jup_sdk::perps::PerpsPosition;

pub async fn get_binance_prompt(
    prediction_type: &PredictionType,
    pair_symbol: &str,
    model: &GeminiModel,
    orderbook_limit: i32,
    maybe_preps_positions: Option<Vec<PerpsPosition>>,
    maybe_timeframe: Option<String>,
) -> anyhow::Result<String> {
    let kline_data_1s = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1s", 1).await?;
    let current_price = kline_data_1s[0].close;

    // Fetch 1m kline data: 500 candles = ~8.3 hours
    let kline_data_1m =
        fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1m", 500 * 3 * 2).await?;
    let price_history_1m_string = serde_json::to_string(&kline_data_1m)?;

    // Fetch 5m kline data: 288 candles = 24h for 1-day short-term analysis
    let kline_data_5m =
        fetch_binance_kline_data::<ConciseKline>(pair_symbol, "5m", 288 * 3).await?;
    let price_history_5m_string = serde_json::to_string(&kline_data_5m)?;

    // Fetch 1h kline data: 168 candles = 7d for 1h signal context
    let kline_data_1h = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1h", 168).await?;
    let price_history_1h_string = serde_json::to_string(&kline_data_1h)?;

    // Fetch 4h kline data: 84 candles = 14d for 4h signals
    let kline_data_4h = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "4h", 84).await?;
    let price_history_4h_string = serde_json::to_string(&kline_data_4h)?;

    // Fetch 1d kline data: 100 candles = ~3m for long-term context
    let kline_data_1d = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1d", 100).await?;
    let price_history_1d_string = serde_json::to_string(&kline_data_1d)?;

    let price_history = PriceHistory {
        price_history_1m: Some(price_history_1m_string),
        price_history_5m: Some(price_history_5m_string),
        price_history_1h: Some(price_history_1h_string),
        price_history_4h: Some(price_history_4h_string),
        price_history_1d: Some(price_history_1d_string),
    };

    let orderbook = fetch_orderbook_depth(pair_symbol, orderbook_limit).await?;

    // --- Build Prompt for Gemini API ---
    println!("Building prompt for Gemini API...");
    let prompt = build_prompt(
        prediction_type,
        model,
        1000f64,
        pair_symbol,
        current_price,
        Some(price_history),
        orderbook,
        maybe_preps_positions,
        maybe_timeframe,
    );

    println!("{prompt:?}");
    Ok(prompt)
}
