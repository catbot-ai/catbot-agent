use crate::predictions::prediction_types::PredictionType;
pub const PREFIX_INSTRUCTION: &str = r#"
- Perform technical analysis on price histories (1m, 5m, 1h, 4h, 1d) and order book volume:
  - Use 1m, 5m, and 1h equally for short-term signals (intraday focus). Prioritize 1m if rapid momentum shifts occur.
  - Use 4h and 1d only to confirm broader trends, not to override short-term signals unless long-term volume is unusually high (>2x 10-period average).
- Detect momentum and reversals with key indicators:
  - Bullish: Stochastic RSI <20, price near lower Bollinger Band, or rising bid volume.
  - Bearish: Stochastic RSI >80, price near upper Bollinger Band, or rising ask volume.
- Analyze bid/ask volume and price action across all timeframes:
  - Bullish signals: Bids outpace asks or price-volume divergence supports upside.
  - Bearish signals: Asks outpace bids or selling volume spikes at resistance.
- Confidence (0.0–1.0):
  - Base at 0.5, +0.1 per aligned indicator (e.g., RSI, volume), -0.1 per conflict.
  - Suggest trades only if confidence ≥0.6; otherwise, recommend monitoring.
- Focus on relative indicators (e.g., % changes, z-scores) over absolute levels to avoid overfitting.
"#;

pub const SCHEMA_INSTRUCTION: &str = r#"
- Kline data is provided as an array of arrays: [[open_time, open, high, low, close, volume, close_time], ...].
- Timestamps are in milliseconds (e.g., 1741870260000); prices and volume are floats (e.g., 123.45, 1000.5).
- Assume data is sorted by open_time ascending and matches the requested timeframe (e.g., 1m, 5m, 1h).
"#;

pub const TRADE_INSTRUCTION: &str = r#"
- Predict the next price top or bottom using:
  - Bollinger Bands for overbought/oversold levels.
  - Moving Average crossovers (e.g., 9-day SMA vs. 21-day SMA).
  - Recent price action and volume trends from order book.
- Suggest entry timing based on short-term signals (1m, 5m, 1h) aligning with predicted tops/bottoms.
- Provide target_price with ≥2.5% profit potential:
  - Longs: Above upper Bollinger Band or recent high.
  - Shorts: Below lower Bollinger Band or recent low.
- Include stop_loss to limit risk below profit potential.
"#;

pub const PERPS_INSTRUCTION: &str = r#"
- For existing positions, suggest one of the following actions based on current momentum, price action, and volume, ensuring logical risk management:
    - 'Hold': If short-term momentum aligns with the position’s side (e.g., bearish for shorts, bullish for longs).
    - 'Increase': If at least two short-term indicators (e.g., Stochastic RSI, volume, price action) strongly confirm the position’s direction and confidence exceeds 0.7.
    - 'Close': If short-term signals contradict the position’s side or the position nears its target or stop_loss.
    - 'Reverse': If short-term signals strongly oppose the position’s side and indicate a clear reversal (e.g., Stochastic RSI crossing 20 from below for a short with rising bid volume), suggest closing the current position and opening an opposite position with new entry_price, target_price, and stop_loss.
    - Ensure stop_loss values are logically set:
    - For longs, set stop_loss 1-2% below the entry_price or nearest support (e.g., below the lower Bollinger Band or 9-day SMA if price is volatile).
    - For shorts, set stop_loss 1-2% above the entry_price or nearest resistance (e.g., above the upper Bollinger Band or 9-day SMA).
"#;

// TODO: maybe_timeframe
pub const GRAPH_INSTRUCTION: &str = r#"
- Predict 24 klines value for 1h timeframe base on technical analysis and vibe.
- Ensure that suggested long/short signals is matched predicted klines time and value.
"#;

pub const SUFFIX_INSTRUCTION: &str = r#"
- Be concise, think step by step.
- Must generate valid JSON output.
"#;

pub fn get_instruction(
    prediction_type: &PredictionType,
    maybe_timeframe: Option<String>,
) -> String {
    match prediction_type {
        PredictionType::Suggestions => {
            format!(
                r#"{PREFIX_INSTRUCTION}{SCHEMA_INSTRUCTION}{TRADE_INSTRUCTION}{PERPS_INSTRUCTION}{SUFFIX_INSTRUCTION}"#
            )
        }
        PredictionType::GraphPredictions => {
            format!(
                r#"{PREFIX_INSTRUCTION}{SCHEMA_INSTRUCTION}{TRADE_INSTRUCTION}{GRAPH_INSTRUCTION}{SUFFIX_INSTRUCTION}"#
            )
        }
    }
}
