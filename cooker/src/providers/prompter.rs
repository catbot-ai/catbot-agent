use chrono::Utc;
use common::OrderBook;
use jup_sdk::perps::PerpsPosition;

use common::transforms::numbers::btree_map_to_csv;
use common::transforms::numbers::group_by_fractional_part;
use common::transforms::numbers::top_n_bids_asks;
use common::transforms::numbers::FractionalPart;

use crate::predictions::prediction_types::PredictionType;
use crate::providers::instructions::get_instruction;
use crate::providers::schemas::get_perps_position_schema;
use crate::providers::schemas::get_schema_instruction;

use super::core::PriceHistory;

#[allow(clippy::too_many_arguments, unused)]
pub fn build_prompt<T>(
    prediction_type: &PredictionType,
    model: &T,
    fund_usd: f64,
    pair_symbol: &str,
    current_price: f64,
    price_history: Option<PriceHistory>,
    orderbook: OrderBook,
    maybe_preps_positions: Option<Vec<PerpsPosition>>,
    maybe_timeframe: Option<String>,
) -> String {
    let now_utc = Utc::now();
    let current_datetime = now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let current_timestamp = now_utc.timestamp_millis();

    // TODO: Better handle binance_pair_symbol
    let pair_symbol = pair_symbol.replace("_", "");
    let symbol = pair_symbol.split("USDT").next().unwrap_or(&pair_symbol);

    let (grouped_one_bids, grouped_one_asks) =
        group_by_fractional_part(&orderbook, FractionalPart::One);

    // Limit 10
    let top_bids_price_amount = top_n_bids_asks(&grouped_one_bids, 10, false);
    let top_asks_price_amount = top_n_bids_asks(&grouped_one_asks, 10, true);

    let grouped_bids_string = btree_map_to_csv(&grouped_one_bids);
    let grouped_asks_string = btree_map_to_csv(&grouped_one_asks);

    let min_profit = fund_usd * 0.025;

    // Position
    let (maybe_preps_positions_string, maybe_position_schema) =
        get_perps_position_schema(maybe_preps_positions);

    // Instructions
    let instruction = get_instruction(prediction_type, maybe_timeframe);
    let schema_instruction = get_schema_instruction(
        prediction_type,
        current_price,
        symbol,
        top_bids_price_amount,
        top_asks_price_amount,
        maybe_position_schema,
    );

    let price_history_string = price_history
        .as_ref()
        .map_or(String::new(), |history| history.to_formatted_string());

    // Consolidate
    format!(
        r#"Analyze {symbol} for price movement in the next 4 hours using:

## Input Data:
fund_usd={fund_usd}
current_datetime={current_datetime}
current_timestamp={current_timestamp}
current_price={current_price}

## Open positions:
{maybe_preps_positions_string}

## Historical Data:
{price_history_string}

## Consolidated Data:

**Bids:**
{grouped_bids_string}

**Asks:**
{grouped_asks_string}

## Instructions:
{instruction}

## Output:
```json
{schema_instruction}
```
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::gemini::GeminiModel;
    use anyhow::Result;
    use common::{
        binance::{fetch_binance_kline_data, fetch_orderbook_depth},
        jup::get_preps_position,
        ConciseKline,
    };
    use std::env;
    use tokio;

    #[tokio::test]
    async fn test_build_prompt_stage1_empty_price_history() -> Result<(), Box<dyn std::error::Error>>
    {
        // Define pair symbol
        let binance_pair_symbol = "SOLUSDT";
        let maybe_timeframe = Some("1h".to_string());

        // Fetch 1-second kline data to get current price
        let kline_data_1s =
            fetch_binance_kline_data::<ConciseKline>(binance_pair_symbol, "1s", 1).await?;
        let current_price = kline_data_1s[0].close;

        let kline_data_1h =
            fetch_binance_kline_data::<ConciseKline>(binance_pair_symbol, "1h", 1).await?;
        let price_history_1h_string = serde_json::to_string_pretty(&kline_data_1h)?;

        // Create an empty PriceHistory struct (all fields None)
        let price_history = PriceHistory {
            price_history_1m: None,
            price_history_5m: Some("[]".to_string()),
            price_history_1h: Some(price_history_1h_string),
            price_history_4h: Some("[]".to_string()),
            price_history_1d: Some("[]".to_string()),
        };

        // Fetch orderbook (assuming fetch_orderbook_depth returns OrderBook)
        let orderbook = fetch_orderbook_depth("SOLUSDT", 100).await?;

        // Create a default GeminiModel
        let model = GeminiModel::default();

        // Load environment variables from .env file (optional, handle errors gracefully)
        dotenvy::from_filename(".env").ok(); // Use .ok() to avoid panic if .env is missing
        let wallet_address = env::var("WALLET_ADDRESS").ok(); // Use .ok() to handle missing env var
        let maybe_preps_positions = get_preps_position(wallet_address).await?;

        // Call the refactored build_prompt with Option<PriceHistory>
        let prompt = build_prompt(
            &PredictionType::Suggestions,
            &model,                // Reference to GeminiModel
            1000f64,               // fund_usd
            binance_pair_symbol,   // pair_symbol (e.g., "SOLUSDT")
            current_price,         // current_price
            Some(price_history),   // Option<PriceHistory> with empty data
            orderbook,             // OrderBook
            maybe_preps_positions, // Option<Vec<PerpsPosition>>
            maybe_timeframe,
        );

        // Print the prompt for verification
        println!("{}", prompt);

        Ok(())
    }

    #[tokio::test]
    async fn test_build_prompt_predict_signal_and_candles() -> Result<(), Box<dyn std::error::Error>>
    {
        // Define pair symbol
        let binance_pair_symbol = "SOLUSDT";
        let maybe_timeframe = Some("1h".to_string());

        // Fetch 1-second kline data to get current price
        let kline_data_1s =
            fetch_binance_kline_data::<ConciseKline>(binance_pair_symbol, "1s", 1).await?;
        let current_price = kline_data_1s[0].close;

        let kline_data_1h =
            fetch_binance_kline_data::<ConciseKline>(binance_pair_symbol, "1h", 1).await?;
        let price_history_1h_string = serde_json::to_string_pretty(&kline_data_1h)?;

        // Create an empty PriceHistory struct (all fields None)
        let price_history = PriceHistory {
            price_history_1m: None,
            price_history_5m: Some("[]".to_string()),
            price_history_1h: Some(price_history_1h_string),
            price_history_4h: Some("[]".to_string()),
            price_history_1d: Some("[]".to_string()),
        };

        // Fetch orderbook (assuming fetch_orderbook_depth returns OrderBook)
        let orderbook = fetch_orderbook_depth(binance_pair_symbol, 100).await?;

        // Create a default GeminiModel
        let model = GeminiModel::default();

        // Call the refactored build_prompt with Option<PriceHistory>
        let prompt = build_prompt(
            &PredictionType::GraphPredictions,
            &model,              // Reference to GeminiModel
            1000f64,             // fund_usd
            binance_pair_symbol, // pair_symbol (e.g., "SOLUSDT")
            current_price,       // current_price
            Some(price_history), // Option<PriceHistory> with empty data
            orderbook,           // OrderBook
            None,                // Option<Vec<PerpsPosition>>
            maybe_timeframe,
        );

        // Print the prompt for verification
        println!("{}", prompt);

        Ok(())
    }
}
