use crate::predictions::prediction_types::PredictionType;

pub const PREFIX_INSTRUCTION: &str = r#"
- Perform technical analysis on price histories (1m, 5m, 1h, 4h, 1d) and order book volume:
  - Use 1m, 5m, and 1h equally for short-term signals (intraday focus). Prioritize 1m if rapid momentum shifts occur.
  - Use 4h and 1d to confirm broader trends or detect weekly patterns; weight 4h/1d higher only if volume exceeds 2x 10-period average and short-term signals (1m, 5m, 1h) do not strongly contradict.
- Detect momentum and reversals with key indicators:
  - Bullish: Stochastic RSI <20, price near lower Bollinger Band, rising bid volume, or EMA (9) crosses above EMA (21).
  - Bearish: Stochastic RSI >80, price near upper Bollinger Band, rising ask volume, or EMA (9) crosses below EMA (21).
- Use Fibonacci retracement/extension levels on 4h and 1d timeframes to identify key support/resistance zones:
  - Bullish: Target 61.8% or 100% extension above recent swing high if momentum confirms.
  - Bearish: Target 61.8% or 100% retracement below recent swing low if volume supports.
- Analyze bid/ask volume and price action across all timeframes:
  - Bullish signals: Bids outpace asks or price-volume divergence supports upside.
  - Bearish signals: Asks outpace bids or selling volume spikes at resistance.
- Account for weekly cycles and news events:
  - Increase confidence (+0.1) for bullish signals on historically strong days (e.g., Wednesday) or post-news spikes (e.g., 8:00 PM GMT+0).
  - Decrease confidence (-0.1) for trades against weekly slowdowns (e.g., Friday to Sunday) unless short-term volume contradicts.
  - If news context is unavailable, assume typical volatility spikes at 8:00 PM GMT+0 and adjust entry/target timing accordingly.
- Analyze historical volatility spikes (e.g., periods with >2x average ATR or volume) on 4h and 1d timeframes. Adjust entry and target timing to avoid whipsaws during spikes unless momentum aligns with the trade direction, in which case prioritize breakout entries.
- Confidence (0.0–1.0):
  - Base at 0.5, +0.1 per aligned indicator (e.g., RSI, volume, EMA, Fibonacci), -0.1 per conflict.
  - Suggest trades only if confidence ≥0.6; for 'Hold' on existing positions, require confidence ≥0.7 to ensure strong alignment.
- Focus on relative indicators (e.g., % changes, z-scores) over absolute levels to avoid overfitting.
"#;

pub const INPUT_INSTRUCTION: &str = r#"
- Kline data is provided as a CSV format.
- Timestamps are in milliseconds (e.g., 1741870260000); prices and volume are floats (e.g., 123.45, 1000.5).
- Assume data is sorted by open_time ascending and matches the requested timeframe (e.g., 1m, 5m, 1h).
"#;

pub const MAIN_TRADE_INSTRUCTION: &str = r#"
- Predict the next price top or bottom using:
  - Bollinger Bands for overbought/oversold levels.
  - EMA crossovers (e.g., 9-period EMA vs. 21-period EMA) across all timeframes.
  - Fibonacci levels (e.g., 61.8% retracement for shorts, 100% extension for longs) aligned with recent swing highs/lows.
  - Recent price action and volume trends from order book.
- Suggest entry timing based on short-term signals (1m, 5m, 1h) aligning with predicted tops/bottoms.
- Provide target_price with ≥2.5% profit potential:
  - Longs: Above upper Bollinger Band, recent high, or Fibonacci 61.8%/100% extension.
  - Shorts: Below lower Bollinger Band, recent low, or Fibonacci 61.8%/100% retracement.
- Include stop_loss to limit risk below profit potential.
"#;

pub const SUB_PERPS_INSTRUCTION: &str = r#"
- Ensure about open position side is "long" or "short" before made a suggestion.
- For existing positions, suggest one of the following actions based on current momentum, price action, and volume, ensuring logical risk management:
    - 'Hold': If short-term momentum clearly aligns with the position’s side (e.g., bullish for longs with Stochastic RSI <20, rising bid volume, or EMA (9) above EMA (21); bearish for shorts with Stochastic RSI >80, rising ask volume, or EMA (9) below EMA (21)). Require confidence ≥0.7 and at least two confirming indicators. Do not suggest 'Hold' if momentum is mixed or opposes the position’s side.
    - 'Increase': If at least two short-term indicators (e.g., Stochastic RSI, volume, price action, EMA crossover) strongly confirm the position’s direction and confidence exceeds 0.7.
    - 'Close': If short-term signals oppose the position’s side (e.g., bearish signals for a long position with Stochastic RSI >80 or rising ask volume) or the position nears its target, stop_loss, or liquidation risk.
    - 'Reverse': If short-term signals strongly oppose the position’s side with confidence ≥0.7 (e.g., bearish signals for a long with Stochastic RSI >80 and rising ask volume), suggest closing the current position and opening an opposite position with new entry_price, target_price, and stop_loss.
    - Ensure stop_loss values are logically set:
    - For longs, set stop_loss 1-2% below the entry_price or nearest support (e.g., below the lower Bollinger Band, 9-period EMA, or Fibonacci 38.2% level if price is volatile).
    - For shorts, set stop_loss 1-2% above the entry_price or nearest resistance (e.g., above the upper Bollinger Band, 9-period EMA, or Fibonacci 38.2% level).
"#;

pub const SUB_GRAPH_INSTRUCTION: &str = r#"
- Predict 24 klines value for 1h timeframe based on technical analysis and vibe.
- Ensure that suggested long/short signals match predicted klines time and value.
"#;

pub const SUFFIX_INSTRUCTION: &str = r#"
- Be concise, think step by step.
- Must generate valid JSON output.
- Ensure summary suggestion aligns with the existing position’s side (e.g., 'Hold long position' for longs, 'Hold short position' for shorts) unless suggesting 'Close' or 'Reverse'.
"#;

pub fn get_instruction(prediction_type: &PredictionType, _timeframe: String) -> String {
    match prediction_type {
        PredictionType::TradingPredictions => {
            format!(
                r#"{PREFIX_INSTRUCTION}{INPUT_INSTRUCTION}{MAIN_TRADE_INSTRUCTION}{SUB_PERPS_INSTRUCTION}{SUFFIX_INSTRUCTION}"#
            )
        }
        PredictionType::GraphPredictions => {
            format!(
                r#"{PREFIX_INSTRUCTION}{INPUT_INSTRUCTION}{MAIN_TRADE_INSTRUCTION}{SUB_GRAPH_INSTRUCTION}{SUFFIX_INSTRUCTION}"#
            )
        }
    }
}
