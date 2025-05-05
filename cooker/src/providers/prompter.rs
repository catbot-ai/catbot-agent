use chrono::Utc;
use common::binance::get_token_and_pair_symbol_usdt;
use common::OrderBook;
use common::TradingContext;

use common::transforms::numbers::btree_map_to_csv;
use common::transforms::numbers::group_by_fractional_part;
use common::transforms::numbers::FractionalPart;

use crate::predictions::prediction_types::PredictionType;
use crate::providers::instructions::get_instruction;
use crate::providers::schemas::get_perps_position_schema;
use crate::providers::schemas::get_schema_instruction;

#[allow(clippy::too_many_arguments, unused)]
pub fn build_prompt<T>(
    prediction_type: &PredictionType,
    model: &T, // Model is generic, kept for potential future use/type constraints
    context: TradingContext,
    historical_data_content: String,
    orderbook: OrderBook,
) -> String {
    // Context
    let current_price = context.current_price;

    // Time
    let now_utc = Utc::now();
    let current_datetime = now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let current_timestamp = now_utc.timestamp_millis();

    // Handle binance_pair_symbol
    let pair_symbol = context.pair_symbol.clone();
    let (token_symbol, _binance_pair_symbol) = get_token_and_pair_symbol_usdt(&pair_symbol); // Use _ if binance_pair_symbol not needed directly here

    // Order Book Processing
    let (grouped_one_bids, grouped_one_asks) =
        group_by_fractional_part(&orderbook, FractionalPart::One);

    // Convert grouped order book data to CSV (limited to top 10 for clarity if needed, or full)
    // For the prompt, let's use the full grouped data for now, matching the original code
    let grouped_bids_string = btree_map_to_csv(&grouped_one_bids);
    let grouped_asks_string = btree_map_to_csv(&grouped_one_asks);

    // If you wanted top N instead:
    // let top_bids_map = top_n_bids_asks(&grouped_one_bids, 10, false);
    // let top_asks_map = top_n_bids_asks(&grouped_one_asks, 10, true);
    // let grouped_bids_string = btree_map_to_csv(&top_bids_map);
    // let grouped_asks_string = btree_map_to_csv(&top_asks_map);;

    // Positions
    let (maybe_preps_positions_string, maybe_position_schema) =
        get_perps_position_schema(context.maybe_preps_positions);

    // Instructions
    let instruction = get_instruction(prediction_type, context.interval);
    let schema_instruction =
        get_schema_instruction(prediction_type, &pair_symbol, maybe_position_schema);

    // historical_data_content is now the pre-formatted string passed as input
    // Ensure it's not empty or provide a placeholder if it might be.
    let final_historical_data = if historical_data_content.trim().is_empty() {
        "No historical data provided or generated.".to_string()
    } else {
        historical_data_content // Use the provided string directly
    };

    // Consolidate into the final prompt
    format!(
        r#"Analyze {pair_symbol} for price movement in the next 4 hours using:

## Input Data:
token_symbol={token_symbol}
current_datetime={current_datetime}
current_timestamp={current_timestamp}
current_price={current_price}

## Open Positions:
{maybe_preps_positions_string}

## Historical Data:
{final_historical_data}

## Consolidated Order Book Data (Grouped by 1.0):

**Bid:**
```csv
{grouped_bids_string}
```

**Asks:**
```csv
{grouped_asks_string}
```

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
    use crate::providers::gemini::GeminiModel; // Assuming GeminiModel is defined elsewhere
    use anyhow::{Context as AnyhowContext, Result}; // Add alias for Context trait
    use common::{
        binance::{fetch_binance_kline_usdt, fetch_orderbook_depth_usdt},
        jup::get_preps_position,
        transforms::csv::PriceHistoryBuilder, // Import the builder
        ConciseKline,
        TradingContext,
    };
    use std::env;
    use tokio;

    // Helper function to create formatted historical data string using PriceHistoryBuilder
    async fn build_historical_data_report(
        pair_symbol: &str,
        // Optional: Add specific intervals if needed for testing different scenarios
        kline_intervals: &[&str],
        stoch_rsi_intervals: &[&str],
        latest_bb_ma_intervals: &[&str],
    ) -> Result<String> {
        // Use default intervals similar to get_binance_prompt for consistency
        // let kline_intervals = ["5m:100", "15m:100", "1h:100", "4h:100", "1d:100"]; // Reduced limits for testing speed
        // let stoch_rsi_intervals = ["4h:100"];
        // let latest_bb_ma_intervals = ["4h:100"];

        println!(
            "Building historical data report for {} using PriceHistoryBuilder...",
            pair_symbol
        );

        // Instantiate and configure the builder
        let full_report = PriceHistoryBuilder::new(pair_symbol, 100) // Base limit (can be overridden per item)
            .with_klines(kline_intervals)
            .with_stoch_rsi(stoch_rsi_intervals)
            .with_latest_bb_ma(latest_bb_ma_intervals)
            .build()
            .await
            .with_context(|| format!("Failed to build historical report for {}", pair_symbol))?;

        Ok(full_report)
    }

    #[tokio::test]
    async fn test_build_prompt_trading_prediction_with_builder(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Define pair symbol
        let token_symbol = "SOL".to_string();
        let pair_symbol = format!("{token_symbol}_USDT");
        let interval = "1h".to_string(); // Example interval for context (instruction generation)

        // Fetch 1-second kline data to get current price
        println!("Fetching current price for {}...", pair_symbol);
        let kline_data_1s =
            fetch_binance_kline_usdt::<ConciseKline>(&token_symbol, "1s", 1).await?;
        let current_price = kline_data_1s
            .first()
            .map(|k| k.close)
            .ok_or("Failed to get current price from 1s kline data")?; // Ensure price is available

        // Load environment variables from .env file
        dotenvy::dotenv().ok(); // Load .env
        let wallet_address = env::var("WALLET_ADDRESS").ok();

        // Fetch positions (handle potential errors or None result)
        let maybe_preps_positions = if let Some(addr) = wallet_address {
            println!("Fetching positions for wallet...");
            get_preps_position(Some(addr)).await.ok().flatten()
        } else {
            println!("WARN: WALLET_ADDRESS not set. Skipping position fetching.");
            None
        };

        // Context
        let context = TradingContext {
            token_symbol: token_symbol.clone(),
            pair_symbol: pair_symbol.clone(), // Builder uses this pair_symbol
            interval: interval.clone(),
            current_price,
            maybe_preps_positions,
            maybe_trading_predictions: None,
            kline_intervals: ["1h:24".to_string()].to_vec(),
            stoch_rsi_intervals: ["4h".to_string()].to_vec(),
            latest_bb_ma_intervals: ["1h".to_string(), "4h".to_string()].to_vec(),
        };

        // --- Generate historical data using PriceHistoryBuilder ---
        let historical_data_content = build_historical_data_report(
            &pair_symbol,
            context
                .kline_intervals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
            context
                .stoch_rsi_intervals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
            context
                .latest_bb_ma_intervals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .await?;
        assert!(
            !historical_data_content.is_empty(),
            "Generated historical data report is empty"
        );

        // Fetch orderbook
        println!("Fetching order book for {}...", pair_symbol);
        let orderbook = fetch_orderbook_depth_usdt(&pair_symbol, 1000).await?; // Use USDT pair symbol

        // Create a model instance (using default for example)
        let model = GeminiModel::default();

        // --- Call build_prompt with the generated historical data string ---
        println!("Building final prompt...");
        let prompt = build_prompt(
            &PredictionType::Trading, // Use Trading prediction type
            &model,
            context,                 // Pass the created context
            historical_data_content, // Pass the string generated by the builder
            orderbook,
        );

        println!("----------------------");
        println!("{prompt}");
        println!("----------------------");
        // --- Assertions (Basic Checks) ---
        println!("Verifying prompt content...");
        assert!(prompt.contains(&format!("Analyze {}", pair_symbol)));
        assert!(prompt.contains("## Historical Data:"));
        assert!(prompt.contains("## Consolidated Order Book Data (Grouped by 1.0):"));
        assert!(prompt.contains("price,cumulative_amount")); // Check for CSV headers in order book
        assert!(prompt.contains("## Instructions:"));

        // Optionally print for manual verification during development
        // println!("--- Trading Prompt (Built with Builder Data) ---");
        // println!("{}", prompt);
        println!("Prompt verification successful.");

        Ok(())
    }

    #[tokio::test]
    #[ignore] // Ignoring by default: depends on external services and takes time
    async fn test_build_prompt_graph_prediction_with_builder(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Define pair symbol
        let token_symbol = "SOL".to_string();
        let pair_symbol = format!("{token_symbol}_USDT");
        let binance_pair_symbol = format!("{token_symbol}USDT");
        let interval = "4h".to_string(); // Example different interval for context

        // Fetch current price
        println!("Fetching current price for {}...", binance_pair_symbol);
        let kline_data_1s =
            fetch_binance_kline_usdt::<ConciseKline>(&binance_pair_symbol, "1s", 1).await?;
        let current_price = kline_data_1s
            .first()
            .map(|k| k.close)
            .ok_or("Failed to get current price from 1s kline data")?;

        // Context (no positions needed/fetched for graph prediction example)
        let context = TradingContext {
            token_symbol: token_symbol.clone(),
            pair_symbol: pair_symbol.clone(), // Builder uses this
            interval: interval.clone(),
            current_price,
            maybe_preps_positions: None, // Explicitly None for this test
            maybe_trading_predictions: None,
            kline_intervals: ["1h:24".to_string()].to_vec(),
            stoch_rsi_intervals: ["4h".to_string()].to_vec(),
            latest_bb_ma_intervals: ["1h".to_string(), "4h".to_string()].to_vec(),
        };

        // --- Generate historical data using PriceHistoryBuilder ---
        let historical_data_content = build_historical_data_report(
            &pair_symbol,
            context
                .kline_intervals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
            context
                .stoch_rsi_intervals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
            context
                .latest_bb_ma_intervals
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .await?;
        assert!(
            !historical_data_content.is_empty(),
            "Generated historical data report is empty"
        );

        // Fetch orderbook
        println!("Fetching order book for {}...", pair_symbol);
        let orderbook = fetch_orderbook_depth_usdt(&pair_symbol, 1000).await?;

        // Create a model instance
        let model = GeminiModel::default();

        // --- Call build_prompt with the generated historical data string ---
        println!("Building final prompt...");
        let prompt = build_prompt(
            &PredictionType::Graph, // Use Graph prediction type
            &model,
            context,
            historical_data_content, // Pass the string generated by the builder
            orderbook,
        );

        // --- Assertions (Basic Checks) ---
        println!("Verifying prompt content...");
        assert!(prompt.contains(&format!("Analyze {}", pair_symbol)));
        assert!(prompt.contains(&format!("interval={}", interval))); // Check correct interval in input data section
        assert!(prompt.contains("## Historical Data:"));
        assert!(prompt.contains("## Consolidated Order Book Data (Grouped by 1.0):"));
        assert!(prompt.contains("price,cumulative_amount"));
        assert!(prompt.contains("## Instructions:"));

        // Optionally print for manual verification
        // println!("--- Graph Prompt (Built with Builder Data) ---");
        // println!("{}", prompt);
        println!("Prompt verification successful.");

        Ok(())
    }
}
