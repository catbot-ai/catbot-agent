use crate::predictions::prediction_types::PredictionType;
pub const TRADE_INSTRUCTION: &str = r#"
- Perform technical analysis on available price histories (1m, 5m, 1h, 4h, 1d) and order book volume. Weight 1m, 5m, and 1h equally for intraday signals unless rapid momentum shifts are detected, in which case prioritize 1m for entry timing. Use 4h and 1d data only to confirm long-term trends, never to override short-term bullish or bearish signals unless long-term volume exceeds 2x the 10-period average.
- For 1h signals (target_datetime within 1–2 hours), prioritize 1m, 5m, and 1h price history to detect short-term momentum shifts. Use 4h and 1d data only if long-term volume is extreme (as defined above).
- Detect potential reversals and momentum shifts using these indicators, focusing on short-term data (1m, 5m, 1h):
  - Bullish reversals: Stochastic RSI <20, price near lower Bollinger Band (z-score < -2), or strong support with rising bid volume and price-volume divergence.
  - Bearish reversals: Stochastic RSI >80, price near upper Bollinger Band (z-score > 2), or strong resistance with rising ask volume and price rejection.
  - Suggest long positions with high confidence (0.7–1.0) when short-term data shows clear bullish patterns (e.g., uptrend with rising Stochastic RSI), and short positions with high confidence when bearish patterns dominate (e.g., rejection at resistance), even if 4h/1d data suggests a different trend.
- Analyze bid/ask volume dynamically across all timeframes (1m, 5m, 1h, 4h, 1d), order book, and recent price action:
  - Prioritize short-term bullish spikes (bids > asks, e.g., bids at current price totaling high volume) or bullish price-volume divergences for 1h long signals.
  - Flag bearish signals when asks significantly outpace bids at resistance or when selling volume spikes on price rejection.
  - If long-term volume exceeds 2x the 10-period average, consider it a potential trend override but maintain short-term signal priority unless volume, price action, and order book data align with the long-term trend.
- Identify recurring price patterns in price history (e.g., spikes, support levels, resistance levels) and align entry_price, target_price, and stop_loss with these patterns using relative indicators (e.g., percentage changes, Bollinger Band z-scores) rather than absolute price levels.
- Calculate confidence scores (0.0–1.0) as follows:
  - Start with a base confidence of 0.5.
  - Increase by 0.1 for each aligned indicator (e.g., Stochastic RSI, volume, price action).
  - Decrease by 0.1 for each conflict (e.g., volume contradicts price movement).
  - Suggest longs or shorts with moderate confidence (0.6–0.7) if short-term data conflicts with long-term trends but prioritize short-term signals.
  - Lower confidence (<0.6) if signals are ambiguous or volume contradicts price action, and suggest monitoring instead of trading.
- Generate trading signals with at least 2.5% profit potential from entry_price to target_price, ensuring:
  - For longs: target_price = max(upper Bollinger Band, recent high) * 1.025.
  - For shorts: target_price = min(lower Bollinger Band, recent low) * 0.975.
  - Stop_loss limits risk to less than the potential profit (e.g., stop_loss risk < 2.5% profit).
- Predict the next price top or bottom using:
  - Bollinger Bands for overbought/oversold conditions (z-score >2 or <-2).
  - Moving Average crossovers (e.g., 9-day SMA crossing above 21-day SMA for bottoms, below for tops).
  - Recent price spikes with confirmation from volume and order book data.
  - Suggest entering positions only when short-term signals (1m, 5m, 1h) align with potential tops/bottoms, even if long-term trends differ.
- When taking profit from a short position, suggest opening a long position at the take-profit value if short-term indicators (1m, 5m, 1h) indicate a bullish reversal (e.g., Stochastic RSI <20, rising bid volume). Provide `new_target_price`, `new_stop_loss`, and `confidence` for the reverse position, ensuring at least 2.5% profit potential and logical risk management.
- In volatile markets (ATR >2% of current price over the last 14 periods), prioritize short-term signals (1m, 5m, 1h) and adjust stop_loss dynamically:
  - For longs: Set stop_loss 2-3% below the nearest support or below the 21-day SMA if wider.
  - For shorts: Set stop_loss 2-3% above the nearest resistance or above the 21-day SMA if wider.
  - Avoid suggesting shorts during clear bullish momentum (e.g., rising Stochastic RSI, high bid volume) or longs during clear bearish momentum (e.g., falling Stochastic RSI, high ask volume).
- Avoid overfitting by focusing on relative indicators (e.g., percentage changes, Bollinger Band z-scores). Lower confidence (<0.6) if volume, price action, or order book data conflicts with the predicted signal, and suggest monitoring instead of trading.
- Generate summary suggestion for positions and signals dynamically using only the provided price history, order book data, and technical analysis rules. Do not replicate or adapt examples from the prompt; instead, base all recommendations on current market conditions and calculated indicators (e.g., Stochastic RSI, Bollinger Bands, volume trends).
- Explicitly explain any discrepancies between signals, positions, and timeframes in the rationale to prevent confusion (e.g., clarify why a short is maintained despite rising bids or neutral long-term trends).
- Don't mixed up signals and positions schema.
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

pub const GRAPH_INSTRUCTION: &str = r#"
- Predict 24 klines value for 1h timeframe base on technical analysis and vibe.
- Ensure that suggested long/short signals is matched predicted klines time and value.
"#;

pub const SUFFIX_INSTRUCTION: &str = r#"
- Be concise, think step by step.
- Must generate valid JSON output.
"#;

pub fn get_instruction(prediction_type: &PredictionType) -> String {
    match prediction_type {
        PredictionType::Suggestions => {
            format!(r#"{TRADE_INSTRUCTION}{PERPS_INSTRUCTION}{SUFFIX_INSTRUCTION}"#)
        }
        PredictionType::GraphPredictions => {
            format!(r#"{TRADE_INSTRUCTION}{GRAPH_INSTRUCTION}{SUFFIX_INSTRUCTION}"#)
        }
    }
}
