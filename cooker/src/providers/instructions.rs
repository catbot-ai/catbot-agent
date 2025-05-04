use crate::predictions::prediction_types::PredictionType;

pub const PREFIX_INSTRUCTION: &str = r#"
- Perform technical analysis on price histories (5m, 15m, 1h, 4h, 1d) and order book volume:
  - Use 5m, 15m, and 1h for short-term signals (intraday focus). Weight 5m higher for early reversal detection (e.g., bullish divergences); prioritize 15m for sustained momentum shifts confirmed by volume or price action.
  - Use 4h and 1d to confirm broader trends or detect weekly patterns; weight 4h/1d higher if volume exceeds 1.5x 10-period average or short-term signals (5m, 15m, 1h) align, reducing reliance on contradictory short-term signals.
- Detect momentum and reversals with key indicators:
  - Bullish: Stochastic RSI <30 (or rising from <20), price near or below lower Bollinger Band, rising bid volume >1.2x ask volume, EMA (9) crosses above EMA (21), or MACD line crosses above signal line.
  - Bearish: Stochastic RSI >70 (or falling from >80), price near or above upper Bollinger Band, rising ask volume >1.2x bid volume, EMA (9) crosses below EMA (21), or MACD line crosses below signal line.
- Use Fibonacci retracement/extension levels on 4h and 1d intervals to identify key support/resistance zones:
  - Bullish: Target 61.8%, 100%, or 161.8% extension above recent swing high if momentum confirms; consider 38.2% retracement as support for entries.
  - Bearish: Target 61.8%, 100%, or 161.8% retracement below recent swing low if volume supports; consider 38.2% extension as resistance for entries.
- Analyze bid/ask volume and price action across all intervals:
  - Bullish signals: Bids outpace asks by >1.2x, price-volume divergence supports upside, or buying volume spikes at support.
  - Bearish signals: Asks outpace bids by >1.2x, price-volume divergence supports downside, or selling volume spikes at resistance.
- Account for weekly cycles and news events:
  - Increase confidence (+0.15) for bullish signals on historically strong days (e.g., Wednesday, Monday) or post-news spikes (e.g., 8:00 PM GMT+0) if price action confirms.
  - Decrease confidence (-0.1) for trades against weekly slowdowns (e.g., Friday to Sunday) unless short-term volume >1.5x average or 5m/15m indicators strongly align.
- Incorporate Summary of Market Events (UTC):

Time_UTC,Event,Note
00:00,Tokyo Open,"Potential Reversal/Gap Fill Zone"
01:30,China Open (SSE/SZSE),"Asia Sentiment Driver"
03:30,China Lunch Break Start,"Liquidity Dip"
05:00,China Re-Opens (Post-Lunch),"Afternoon Session Start"
06:00,Tokyo Close,"Local Session End"
07:00,China Close (SSE/SZSE),"Influences EU Open"
07:00,EU Open (Lon/Fra),"Coincides w/ China Close; Volatility Watch"
10:00,Mid-EU / US Pre-Market,"Trend Watch / Potential Reversal Zone"
13:30,US Open / EU Overlap,"Peak Liquidity/Activity"
~15:00,Pre-EU Close / US Midday,"Potential Reversal Zone"
15:30,EU Close (Lon/Fra),"Overlap Ends / Final Moves Watch"
20:00,US Close,"High Caution/Sharp Moves/Gap Risk Zone"

- Adjust confidence and timing based on market events:
  - Increase confidence (+0.15) for signals aligning with high-activity periods (e.g., 13:30 UTC US Open, 07:00 UTC EU Open) if volume or momentum supports; emphasize bullish signals during uptrend confirmation.
  - Decrease confidence (-0.1) during low-liquidity or high-risk periods (e.g., 03:30 UTC China Lunch Break, 20:00 UTC US Close) unless short-term indicators (5m, 15m) strongly contradict with volume >1.5x average.
  - Shift entry/target timing to avoid reversal zones (e.g., ~15:00 UTC Pre-EU Close, 00:00 UTC Tokyo Open) unless breakout momentum is confirmed with volume >1.5x average.
- Analyze historical volatility spikes (e.g., periods with >1.5x average ATR or volume) on 4h and 1d intervals. Adjust entry and target timing to avoid whipsaws during spikes unless momentum aligns with the trade direction, in which case prioritize breakout entries with higher targets.
- Confidence (0.0–1.0):
  - Base at 0.5, +0.1 per aligned indicator (e.g., RSI, volume, EMA, MACD, Fibonacci), -0.05 per conflict to reduce signal suppression.
  - Include weekly cycle and market event adjustments: +0.15 for bullish signals during high-activity periods or uptrends, -0.1 during low-liquidity or reversal zones unless short-term volume exceeds 1.5x average.
  - Suggest trades if confidence ≥0.55; for 'Hold' on existing positions, require confidence ≥0.65; for 'No Action' if confidence <0.55 with no position.
- Explicitly state confidence in the output JSON under a 'confidence' key.
- Focus on relative indicators (e.g., % changes, z-scores) over absolute levels to avoid overfitting.
- Ensure summary suggestion aligns with the existing position’s side (e.g., 'Hold long position' for longs, 'Hold short position' for shorts) unless suggesting 'Close' or 'Reverse'.
"#;

pub const INPUT_INSTRUCTION: &str = r#"
- Kline data is provided as a CSV format.
- Timestamps are in milliseconds (e.g., 1741870260000); prices and volume are floats (e.g., 123.45, 1000.5).
- Assume data is sorted by open_time ascending and matches the requested interval (e.g., 5m, 15m, 1h).
"#;

pub const MAIN_TRADE_INSTRUCTION: &str = r#"
**Structured Trade Evaluation**
Before making your final trading recommendation, perform the following structured evaluation:
1. **Viability Assessment**:
   - List up to three reasons why this trade is viable (e.g., specific indicator signals, price levels, volume patterns).
   - List up to two reasons why this trade might not be viable (e.g., conflicting indicators, market conditions, liquidity issues).
2. **Logical Integrity Check**:
   - For the most compelling supporting factor, provide evidence from the data that confirms it and state any assumptions you are making.
   - For the most significant risk factor, provide evidence and state assumptions.
3. **Market Fit Analysis**:
   - Determine how this trade aligns with the current broader market trends (e.g., is it with the trend, against it, in a ranging market?).
   - Consider if this trade fits with your typical trading strategy (e.g., day trading, swing trading, position trading).
4. **Implementation Requirements**:
   - Ensure the trade can be executed with the available data and tools:
     - Key resources: Real-time data, order book, historical data.
     - Critical capabilities: Indicator analysis, entry/target/stop-loss calculation.
     - Time to revenue: Expected holding period.
     - Initial investment: Capital required for the trade.
5. **Final Recommendation**:
   - Based on the above analysis, decide on the trading action (e.g., Buy, Sell, Hold, Close, Reverse).
   - Set entry_price, target_price, and stop_loss accordingly.
   - Assign a confidence level (0.0–1.0) based on the strength of supporting factors and risks.
In your output JSON, include in the 'rationale' field a summary of this structured evaluation, highlighting key points from each step.
"#;

pub const SUB_PERPS_INSTRUCTION: &str = r#"
- Ensure the open position side is identified as "long" or "short" before making a suggestion.
- If no position exists (positions is null):
    - Suggest 'Buy' or 'Sell' if confidence ≥0.55 with at least two confirming indicators (e.g., Stochastic RSI, volume trends, EMA crossovers, MACD, Fibonacci levels) and provide entry_price, target_price, and stop_loss.
    - Suggest 'No Action' if confidence <0.55 or signals are mixed/insufficient.
- For existing positions, suggest one of the following actions based on current momentum, price action, and volume, with logical risk management:
    - 'Hold': If short-term momentum clearly aligns with the position’s side (e.g., bullish for longs, bearish for shorts) with confidence ≥0.65 and at least two confirming indicators (e.g., Stochastic RSI, volume trends, EMA crossovers, MACD). Avoid 'Hold' if momentum is mixed or opposes the position.
    - 'Increase': If at least three short-term indicators strongly confirm the position’s direction (e.g., rising momentum, favorable volume, price action, MACD crossover) with confidence >0.75.
    - 'Close': If short-term signals oppose the position’s side (e.g., bearish signals for longs, bullish for shorts), or the position nears its target, stop-loss, or liquidation risk.
    - 'Reverse': If short-term signals strongly oppose the position’s side with confidence ≥0.65, suggest closing the current position and opening an opposite one with a new entry_price, target_price, and stop_loss based on current market conditions.
- Set stop_loss values to manage risk effectively:
  - Base stop_loss on volatility (e.g., 1.5x ATR for ranging markets, 2x ATR for trending markets), support/resistance levels, and technical indicators (e.g., Bollinger Bands, Fibonacci).
  - For shorts, set stop_loss above key resistance levels (e.g., upper Bollinger Band, recent highs, Fibonacci levels, order book ask clusters), adding a 1.5x ATR buffer to avoid whipsaws. Never place stop_loss at or below resistance—it must clear the resistance zone by at least 1% or 1.5x ATR, whichever is larger.
  - For longs, set stop_loss below key support levels (e.g., lower Bollinger Band, recent lows, Fibonacci levels, order book bid clusters), adding a 1.5x ATR buffer.
  - Limit maximum loss to 15-20% of position value for new trades unless higher risk is justified by volatility >1.5x ATR or confidence >0.85 with three confirming indicators.
  - Ensure stop_loss is well below liquidation_price (if provided), with a buffer of 30-40% of the distance to liquidation.
  - Cross-check stop_loss against recent price action (e.g., last 24 hours) to avoid placing it at levels recently hit by spikes.
- Generate re-entry signals after a stop-out:
  - If price reverses in the direction of the original position after hitting stop_loss, generate a new signal if:
    - For shorts: Price crosses back below key resistance with confirming indicators (e.g., Stochastic RSI <70, rising ask volume, EMA(9) < EMA(21)).
    - For longs: Price crosses back above key support with confirming indicators (e.g., Stochastic RSI >30, rising bid volume, EMA(9) > EMA(21)).
    - Confidence ≥0.55 with at least two confirming indicators.
  - Use the same target_price if still valid, or adjust based on updated Fibonacci levels and volatility.
"#;

pub const SUB_GRAPH_INSTRUCTION: &str = r#"
- Predict 24 klines value for 1h interval based on technical analysis and market momentum:
  - Use 5m and 15m for short-term trend confirmation, 4h and 1d for broader context.
  - Incorporate bullish signals (e.g., MACD crossover, volume spikes) to balance predictions.
- Ensure that suggested long/short signals match predicted klines time and value.
"#;

pub const SUB_CONSOLIDATE_INSTRUCTION: &str = r#"
- Focus on key indicators like price action, moving averages, Bollinger Bands, MACD, Stochastic RSI, and volume.
- Consider the confidence level (0.65) and rationale provided in the signals.
- If the charts align with the suggested bias (bullish or bearish) and the entry price (e.g., 130.55) is reasonable given current price (e.g., 130.39), confirm the trade.
- If there are discrepancies (e.g., conflicting signals in the charts, low confidence, or unfavorable risk-reward ratio <1.5:1), reject the trade.
- Return your decision using the `execute_trade_decision` function with the following parameters:
  - `pair_symbol`: "SOL_USDT"
  - `should_trade`: true or false (whether to execute the trade)
  - `rationale`: A brief explanation of your decision

### Tasks
1. Analyze the 15m, 1h, 4h, and 1d charts to confirm the trends, resistance/support levels, and indicator signals (e.g., MACD, Stochastic RSI, Bollinger Bands, volume).
2. Cross-reference the chart analysis with the provided summary and signals.
3. Decide whether to execute the suggested trade (long/short SOL_USDT at X with a target of Y and stop-loss at Z).
4. Use the provided function `execute_trade_decision` to return your decision.
"#;

pub const SUFFIX_INSTRUCTION: &str = r#"
- Be concise, think step by step.
- Must generate valid JSON output with clear rationale for bullish/bearish suggestions.
"#;

pub fn get_instruction(prediction_type: &PredictionType, _interval: String) -> String {
    match prediction_type {
        PredictionType::Trading => {
            format!(
                r#"{PREFIX_INSTRUCTION}{INPUT_INSTRUCTION}{MAIN_TRADE_INSTRUCTION}{SUB_PERPS_INSTRUCTION}{SUFFIX_INSTRUCTION}"#
            )
        }
        PredictionType::Graph => {
            format!(
                r#"{PREFIX_INSTRUCTION}{INPUT_INSTRUCTION}{MAIN_TRADE_INSTRUCTION}{SUB_GRAPH_INSTRUCTION}{SUFFIX_INSTRUCTION}"#
            )
        }
        PredictionType::Rebalance => {
            format!(
                r#"{PREFIX_INSTRUCTION}{INPUT_INSTRUCTION}{MAIN_TRADE_INSTRUCTION}{SUB_PERPS_INSTRUCTION}{SUB_CONSOLIDATE_INSTRUCTION}{SUFFIX_INSTRUCTION}"#
            )
        }
    }
}
