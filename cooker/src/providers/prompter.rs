use chrono::Utc;
use common::OrderBook;
use jup_sdk::perps::PerpsPosition;

use crate::transforms::numbers::btree_map_to_csv;
use crate::transforms::numbers::group_by_fractional_part;
use crate::transforms::numbers::top_n_bids_asks;
use crate::transforms::numbers::FractionalPart;

use super::core::PriceHistory;

#[allow(clippy::too_many_arguments, unused)]
pub fn build_prompt<T>(
    model: &T,
    fund_usd: f64,
    pair_symbol: &str,
    current_price: f64,
    price_history: Option<PriceHistory>,
    orderbook: OrderBook,
    maybe_preps_positions: Option<Vec<PerpsPosition>>,
) -> String {
    let now_utc = Utc::now();
    let current_datetime = now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let current_timestamp = now_utc.timestamp_millis();

    let symbol = pair_symbol.split("USDT").next().unwrap_or(pair_symbol);

    let (grouped_one_bids, grouped_one_asks) =
        group_by_fractional_part(&orderbook, FractionalPart::One);
    let (grouped_one_tenth_bids, grouped_one_tenth_asks) =
        group_by_fractional_part(&orderbook, FractionalPart::OneTenth);

    // Limit 10
    let top_bids_price_amount = top_n_bids_asks(&grouped_one_bids, 10, false);
    let top_asks_price_amount = top_n_bids_asks(&grouped_one_asks, 10, true);

    let grouped_bids_string = btree_map_to_csv(&grouped_one_bids);
    let grouped_asks_string = btree_map_to_csv(&grouped_one_asks);

    let min_profit = fund_usd * 0.025;

    // Positions
    let maybe_preps_positions_string = format!("{:?}", maybe_preps_positions);
    let maybe_position_schema = if let Some(preps_positions) = maybe_preps_positions {
        let mut positions_string = String::from(r#","positions": ["#);
        let positions: Vec<String> = preps_positions
            .iter()
            .map(|pos| {
                format!(
                    r#"{{
        "side": "{}",
        "market_mint": "{}",
        "collateral_mint": "{}",
        "entry_price": {},
        "leverage": {},
        "liquidation_price": {},
        "pnl_after_fees_usd": {},
        "value": {},
        "target_price": {:?},
        "stop_loss": {:?},

        "suggestion": string // Suggestion for this position. e.g. "Hold short position. Consider increasing position at 138.5 with stop loss at 140.5 and taking profit at 135."
        "new_target_price": Option<number>,  // A suggested target price
        "new_stop_loss": Option<number>,  // A suggested stop loss
        "confidence": number    // Confidence score between 0.0 and 1.0
    }}"#,
                    pos.side,
                    pos.market_mint,
                    pos.collateral_mint,
                    pos.entry_price,
                    pos.leverage,
                    pos.liquidation_price,
                    pos.pnl_after_fees_usd,
                    pos.value,
                    pos.target_price,
                    pos.stop_loss,
                )
            })
            .collect();
        if !positions.is_empty() {
            positions_string.push_str(&positions.join(","));
        }
        positions_string.push_str("]\n");
        positions_string
    } else {
        String::from(r#","positions": []"#)
    };

    // Instructions
    let schema_instruction = format!(
        r#"**Instructions:**

**Instructions:**

- Perform technical analysis on available price histories (1m, 5m, 1h, 4h, 1d) and order book volume, prioritizing 1m, 5m, and 1h for intraday signals and using 4h/1d for trend context.
- For 1h signals (target_datetime within 1–2 hours), prioritize 1m, 5m, and 1h price history to detect short-term momentum shifts. Use 4h and 1d data only to confirm long-term trends, never to override short-term bullish or bearish signals unless supported by volume, price action, and order book data.
- Detect potential reversals and momentum shifts using these indicators, focusing on short-term data (1m, 5m, 1h):
  - Bullish reversals: Oversold Stochastic RSI (<20), price near lower Bollinger Band, or strong support with rising bid volume and price-volume divergence.
  - Bearish reversals: Overbought Stochastic RSI (>80), price near upper Bollinger Band, or strong resistance with rising ask volume and price rejection.
  - Suggest long positions with high confidence (0.7–1.0) when short-term data shows clear bullish patterns (e.g., uptrend with rising Stochastic RSI), and short positions with high confidence when bearish patterns dominate (e.g., rejection at resistance), even if 4h/1d data suggests a different trend.
- Analyze bid/ask volume dynamically across all timeframes (1m, 5m, 1h, 4h, 1d), order book, and recent price action:
  - Prioritize short-term bullish spikes (bids > asks, e.g., bids at current price totaling high volume) or bullish price-volume divergences for 1h long signals.
  - Flag bearish signals when asks significantly outpace bids at resistance or when selling volume spikes on price rejection.
- Identify recurring price patterns in price history (e.g., spikes, support levels, resistance levels) and align entry_price, target_price, and stop_loss with these patterns to optimize profit potential and minimize risk, using relative indicators (e.g., percentage changes, Bollinger Band z-scores) rather than absolute price levels.
- Calculate confidence scores (0.0–1.0) based on timeframe alignment:
  - Suggest longs or shorts with moderate confidence (0.6–0.7) if 1m/5m/1h data conflicts with 4h/1d trends, but always prioritize short-term signals unless long-term trends are strongly confirmed by volume, price action, and order book data.
  - Lower confidence (e.g., <0.6) if volume contradicts price movement (e.g., bullish price with high ask volume) or if signals are ambiguous, and suggest monitoring instead of trading.
- For existing positions, suggest one of the following actions based on current momentum, price action, and volume, ensuring logical risk management:
  - 'Hold': If short-term momentum aligns with the position’s side (e.g., bearish for shorts, bullish for longs).
  - 'Increase': If short-term signals (1m, 5m, 1h) strongly confirm the position’s direction and volume supports it (e.g., rising asks for shorts, rising bids for longs).
  - 'Close': If short-term signals contradict the position’s side (e.g., bullish signals for a short) or if the position nears its target or stop loss.
  - 'Reverse': If short-term signals strongly oppose the position’s side and indicate a clear reversal (e.g., bullish reversal for a short, bearish reversal for a long), suggest closing the current position and opening an opposite position with new entry_price, target_price, and stop_loss.
  - Ensure stop_loss values are logically set:
    - For longs, set stop_loss 1-2% below the entry_price or nearest support (e.g., below the lower Bollinger Band or 9-day SMA if price is volatile).
    - For shorts, set stop_loss 1-2% above the entry_price or nearest resistance (e.g., above the upper Bollinger Band or 9-day SMA, not above the current price like a higher arbitrary value).
- Generate trading signals with at least 2.5% profit potential from entry_price to target_price, ensuring:
  - Target_price exceeds the first significant resistance (for longs, e.g., beyond the upper Bollinger Band or recent high) or falls below the first significant support (for shorts, e.g., below the lower Bollinger Band or recent low).
  - Stop_loss limits risk to less than the potential profit (e.g., stop_loss risk < 2.5% profit).
- Predict the next price top (e.g., resistance at upper Bollinger Band or recent high) or bottom (e.g., support at lower Bollinger Band or recent low) using:
  - Bollinger Bands for overbought/oversold conditions.
  - Moving Average crossovers or price rejection at MAs.
  - Recent price spikes (e.g., last high, last low) with confirmation from volume and order book data.
  - Suggest entering positions only when short-term signals (1m, 5m, 1h) align with potential tops/bottoms, even if long-term trends (4h, 1d) differ.
- When taking profit from a short position at a specific value, suggest opening a long position at that take-profit value if short-term indicators (1m, 5m, 1h) indicate a bullish reversal (e.g., oversold Stochastic RSI, rising bid volume). Provide `new_target_price`, `new_stop_loss`, and `confidence` for the reverse position, ensuring at least 2.5% profit potential and logical risk management.
- In volatile markets, prioritize short-term signals (1m, 5m, 1h) to detect rapid momentum shifts. Adjust stop_loss dynamically:
  - For longs, set stop_loss 1-2% below the nearest support or below the 9-day SMA if price is volatile.
  - For shorts, set stop_loss 1-2% above the nearest resistance or above the 9-day SMA.
  - Avoid suggesting shorts during clear bullish momentum (e.g., rising Stochastic RSI, high bid volume) or longs during clear bearish momentum (e.g., falling Stochastic RSI, high ask volume).
- Avoid overfitting by focusing on relative indicators (e.g., percentage changes, Bollinger Band z-scores) rather than absolute price levels. Lower confidence (<0.6) if volume, price action, or order book data conflicts with the predicted signal, and suggest monitoring instead of trading.
- Be concise, think step by step, and explicitly explain any discrepancies between signals, positions, and timeframes in the rationale to prevent confusion (e.g., clarify why a short is maintained despite rising bids or neutral long-term trends).
- Be concise about valid JSON output.

**JSON Output:**
```json
{{
    "summary": {{
        "title": "string", // E.g., "{symbol} Short-term Bearish"
        "price": {current_price},
        "upper_bound": number, // Highest top_3_resistance
        "lower_bound": number, // Lowest top_3_support
        "technical_resistance_4h": number, // From 4h analysis
        "technical_support_4h": number, // From 4h analysis
        "top_bids_price_amount": {top_bids_price_amount:?},
        "top_asks_price_amount": {top_asks_price_amount:?},
        "detail": "string", // <500 chars, include volume and momentum insights
        "suggestion": "string", // E.g., "Short {symbol} at 170.1 if volume confirms resistance"
        "vibe": "string" // E.g., "Bearish 65%", match signal confidence
    }},
    "signals": [{{
        "side": string, // long or shot
        "symbol": "{symbol}",
        "confidence": number, // Confidence about this signal: 0.0-1.0
        "current_price": {current_price},
        "entry_price": number,
        "target_price": number, // >2.5% above entry, beyond first resistance or support
        "stop_loss": number, // The value should less than profit.
        "timeframe": "string", // "1h" or "4h"
        "entry_datetime": "string", // ISO time prediction when to make a trade for this signal, Can be now or in the future date time.
        "target_datetime": "string", // ISO time prediction when to take profit.
        "rationale": "string" // E.g., "4h momentum up, bids outpace asks", "1h rejection at 170, high ask volume"
    }}]{maybe_position_schema}
}}
```
"#
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

{schema_instruction}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        providers::gemini::GeminiModel,
        sources::{
            binance::{fetch_binance_kline_data, fetch_orderbook_depth},
            jup::get_preps_position,
        },
    };
    use anyhow::Result;
    use common::ConciseKline;
    use std::env;
    use tokio;

    #[tokio::test]
    async fn test_build_prompt_stage1_empty_price_history() -> Result<(), Box<dyn std::error::Error>>
    {
        // Define pair symbol
        let pair_symbol = "SOLUSDT";

        // Fetch 1-second kline data to get current price
        let kline_data_1s = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1s", 1).await?;
        let current_price = kline_data_1s[0].close;

        let kline_data_1h = fetch_binance_kline_data::<ConciseKline>(pair_symbol, "1h", 1).await?;
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
            &model,                // Reference to GeminiModel
            1000f64,               // fund_usd
            pair_symbol,           // pair_symbol (e.g., "SOLUSDT")
            current_price,         // current_price
            Some(price_history),   // Option<PriceHistory> with empty data
            orderbook,             // OrderBook
            maybe_preps_positions, // Option<Vec<PerpsPosition>>
        );

        // Print the prompt for verification
        println!("{}", prompt);

        Ok(())
    }
}
