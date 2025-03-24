use jup_sdk::{perps::PerpsPosition, token_registry::get_by_address};

use crate::predictions::prediction_types::PredictionType;

pub fn get_signal_schema(pair_symbol: &str) -> String {
    format!(
        r#""signals": [{{
        "pair_symbol": {pair_symbol},
        "direction": string, // Predicted direction, long or shot
        "confidence": number, // Confidence about this signal: 0.0-1.0
        "entry_price": number, // Suggest entry price, Can be future price.
        "target_price": number, // Suggest target price, Can be future price.
        "stop_loss": number,  // Suggest stop loss, Can be future price.
        "entry_time": number, // Timestamp prediction when to make a trade for this signal, Can be now or in the future.
        "target_time": number, // Timestamp prediction when to take profit.
        "rationale": "string" // Rationale about this signal e.g., "4h momentum up, bids outpace asks", "1h rejection at xxx, high ask volume"
    }}]"#
    )
}

pub fn get_perps_position_schema(
    maybe_preps_positions: Option<Vec<PerpsPosition>>,
) -> (String, String) {
    // Positions
    let maybe_preps_positions_string = if maybe_preps_positions.is_none() {
        String::from("No open positions.")
    } else {
        serde_json::to_string(&maybe_preps_positions).unwrap_or("No open positions.".to_string())
    };

    let maybe_position_schema = if let Some(preps_positions) = maybe_preps_positions {
        let mut positions_string = String::from(r#""positions": ["#);
        let positions: Vec<String> = preps_positions
            .iter()
            .map(|preps_position| {
                let token_symbol = get_by_address(&preps_position.market_mint)
                    .expect("Not support token pair")
                    .symbol
                    .to_string();

                format!(r#"{{
        "token_symbol" : {token_symbol},
        "new_target_price": Option<number>,  // Suggested new target price if adjusting position needed or when target_price is null
        "new_stop_loss": Option<number>,     // Suggested new stop loss if adjusting position needed or when stop_loss is null
        "suggestion": "string", // A concise action (e.g., "Hold", "Increase", "Close", "Reverse")
        "rationale": "string", // A brief explanation for the suggestion
        "confidence": number   // Confidence score between 0.0 and 1.0
    }}"#)
            })
            .collect();
        if !positions.is_empty() {
            positions_string.push_str(&positions.join(","));
        }
        positions_string.push(']');
        positions_string
    } else {
        String::from(
            r#",
    "positions": []"#,
        )
    };

    (maybe_preps_positions_string, maybe_position_schema)
}

pub fn get_schema_instruction(
    prediction_type: &PredictionType,
    pair_symbol: &str,
    maybe_position_schema: String,
) -> String {
    let signal_schema = get_signal_schema(pair_symbol);
    match prediction_type {
        PredictionType::TradingPredictions => format!(
            r#"{{
    "summary": {{
        "technical_resistance_4h": number, // Estimated 4h resistance from provided data.
        "technical_support_4h": number, // Estimated 4h support from provided data.
        "vibe": "string", // Current market vibe e.g., "{pair_symbol} Short-term 65% Bearish"
        "detail": "string", // Trading analysis <500 chars, include volume and momentum insights
        "suggestion": "string" // Suggestion trading action e.g., "Short {pair_symbol} at xxx if volume confirms resistance"
    }},
    {signal_schema},
    {maybe_position_schema}
}}
"#
        ),
        PredictionType::GraphPredictions => format!(
            r#"{{
    {signal_schema},
    "klines": [
        [
            1741870260000,  // Open time: Timestamp in milliseconds when the K-line opens
            "123.45", // Predicted open price: The price at the start of the time interval
            "123.45", // Predicted high price: The highest price during the time interval
            "123.45", // Predicted low price: The lowest price during the time interval
            "123.45", // Predicted close price: The price at the end of the time interval
            "0",   // Predicted  volume: The total trading volume during the time interval
            1741873860000,  // Close time: Timestamp in milliseconds when the K-line closes
        ]
    ]
 }}
"#
        ),
        PredictionType::RebalancePredictions => format!(
            r#"{{
    pair_symbol: {pair_symbol},
    should_trade: boolean, // Whether to execute the trade, true or false
    rationale, // A brief explanation of the decision to trade or not
}}
"#
        ),
    }
}
