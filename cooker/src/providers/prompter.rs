use chrono::Utc;
use common::OrderBook;
use common::TradingContext;

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
    context: TradingContext,
    maybe_price_history: Option<PriceHistory>,
    orderbook: OrderBook,
    fund_usd: f64,
) -> String {
    // Context
    let current_price = context.current_price;

    // Time
    let now_utc = Utc::now();
    let current_datetime = now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let current_timestamp = now_utc.timestamp_millis();

    // TODO: Better handle binance_pair_symbol
    let pair_symbol = context.pair_symbol.clone();
    let binance_pair_symbol = pair_symbol.replace("_", "");
    let symbol = binance_pair_symbol
        .split("USDC")
        .next()
        .unwrap_or(&binance_pair_symbol);

    let (grouped_one_bids, grouped_one_asks) =
        group_by_fractional_part(&orderbook, FractionalPart::One);

    // Limit 10
    let top_bids_price_amount = top_n_bids_asks(&grouped_one_bids, 10, false);
    let top_asks_price_amount = top_n_bids_asks(&grouped_one_asks, 10, true);

    println!("top_bids_price_amount:{top_bids_price_amount:#?}");
    println!("top_asks_price_amount:{top_asks_price_amount:#?}");

    let grouped_bids_string = btree_map_to_csv(&grouped_one_bids);
    let grouped_asks_string = btree_map_to_csv(&grouped_one_asks);

    // TODO: take this to the account
    let min_profit = fund_usd * 0.025;

    // TODO: replace market_mint, collateral_mint with symbol
    // Positions
    let (maybe_preps_positions_string, maybe_position_schema) =
        get_perps_position_schema(context.maybe_preps_positions);

    // Instructions
    let instruction = get_instruction(prediction_type, context.timeframe);
    let schema_instruction =
        get_schema_instruction(prediction_type, &pair_symbol, maybe_position_schema);

    let price_history_string = maybe_price_history
        .as_ref()
        .map_or(String::new(), |history| history.to_formatted_string());

    // Consolidate
    format!(
        r#"Analyze {symbol} for price movement in the next 4 hours using:

## Input Data:
symbol={symbol}
fund_usd={fund_usd}
current_datetime={current_datetime}
current_timestamp={current_timestamp}
current_price={current_price}

## Open positions:
{maybe_preps_positions_string}

## Historical Data in CSV:
{price_history_string}

## Consolidated Data in CSV:

**Bids:**
{grouped_bids_string}

**Asks:**
{grouped_asks_string}

## Instructions:
{instruction}

## Output in JSON:
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
        binance::{fetch_binance_kline_csv, fetch_binance_kline_data, fetch_orderbook_depth},
        jup::get_preps_position,
        ConciseKline,
    };
    use std::env;
    use tokio;

    #[tokio::test]
    async fn test_build_prompt_stage1_empty_price_history() -> Result<(), Box<dyn std::error::Error>>
    {
        // Define pair symbol
        let pair_symbol = "SOL_USDC".to_string();
        let binance_pair_symbol = "SOLUSDC";
        let timeframe = "1h".to_string();

        // Fetch 1-second kline data to get current price
        let kline_data_1s =
            fetch_binance_kline_data::<ConciseKline>(binance_pair_symbol, "1s", 1).await?;
        let current_price = kline_data_1s[0].close;

        // Load environment variables from .env file (optional, handle errors gracefully)
        dotenvy::from_filename(".env").ok(); // Use .ok() to avoid panic if .env is missing
        let wallet_address = env::var("WALLET_ADDRESS").ok(); // Use .ok() to handle missing env var
        let maybe_preps_positions = get_preps_position(wallet_address).await?;

        // Context
        let context = TradingContext {
            pair_symbol,
            timeframe,
            current_price,
            maybe_preps_positions,
        };

        let kline_data_1h = fetch_binance_kline_csv(binance_pair_symbol, "1h", 1).await?;

        // Create an empty PriceHistory struct (all fields None)
        let price_history = PriceHistory {
            price_history_1m: None,
            price_history_5m: Some("[]".to_string()),
            price_history_1h: Some(kline_data_1h),
            price_history_4h: Some("[]".to_string()),
            price_history_1d: Some("[]".to_string()),
        };

        // Fetch orderbook (assuming fetch_orderbook_depth returns OrderBook)
        let orderbook = fetch_orderbook_depth("SOLUSDC", 1000).await?;

        // Create a default GeminiModel
        let model = GeminiModel::default();

        // Call the refactored build_prompt with Option<PriceHistory>
        let prompt = build_prompt(
            &PredictionType::TradingPredictions,
            &model, // Reference to GeminiModel
            context,
            Some(price_history), // Option<PriceHistory> with empty data
            orderbook,           // OrderBook
            1000f64,             // fund_usd
        );

        // Print the prompt for verification
        println!("{}", prompt);

        Ok(())
    }

    #[tokio::test]
    async fn test_build_prompt_predict_signal_and_candles() -> Result<(), Box<dyn std::error::Error>>
    {
        // Define pair symbol
        let pair_symbol = "SOL_USDC".to_string();
        let binance_pair_symbol = "SOLUSDC";
        let timeframe = "1h".to_string();

        // Fetch 1-second kline data to get current price
        let kline_data_1s =
            fetch_binance_kline_data::<ConciseKline>(binance_pair_symbol, "1s", 1).await?;
        let current_price = kline_data_1s[0].close;

        // Context
        let context = TradingContext {
            pair_symbol,
            timeframe,
            current_price,
            maybe_preps_positions: None,
        };

        let kline_data_1h = fetch_binance_kline_csv(binance_pair_symbol, "1h", 1).await?;
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
        let orderbook = fetch_orderbook_depth(binance_pair_symbol, 1000).await?;

        // Create a default GeminiModel
        let model = GeminiModel::default();

        // Call the refactored build_prompt with Option<PriceHistory>
        let prompt = build_prompt(
            &PredictionType::GraphPredictions,
            &model, // Reference to GeminiModel
            context,
            Some(price_history), // Option<PriceHistory> with empty data
            orderbook,           // OrderBook
            1000f64,             // fund_usd
        );

        // Print the prompt for verification
        println!("{}", prompt);

        Ok(())
    }
}
