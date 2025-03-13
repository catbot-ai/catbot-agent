use jup_sdk::perps::PerpsPosition;

use crate::predictions::prediction_types::PredictionType;

pub fn get_perps_position_schema(
    maybe_preps_positions: Option<Vec<PerpsPosition>>,
) -> (String, String) {
    // Positions
    let maybe_preps_positions_string = format!("{:?}", maybe_preps_positions);
    let maybe_position_schema = if let Some(preps_positions) = maybe_preps_positions {
        let mut positions_string = String::from(
            r#",
    "positions": ["#,
        );
        let positions: Vec<String> = preps_positions
            .iter()
            .map(|pos| {
                // Handle target_price: Some(f64) -> number, None -> "null"
                let target_price_str = match pos.target_price {
                    Some(tp) => tp.to_string(),
                    None => "null".to_string(),
                };
                // Handle stop_loss: Some(f64) -> number, None -> "null"
                let stop_loss_str = match pos.stop_loss {
                    Some(sl) => sl.to_string(),
                    None => "null".to_string(),
                };
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
        "target_price": {}, // Current target_price, null if not set
        "stop_loss": {}, // Current stop_loss, null if not set
        "new_target_price": Option<number>,  // Suggested target price if adjusting position
        "new_stop_loss": Option<number>,     // Suggested stop loss if adjusting position
        "suggestion": "string", // A concise action (e.g., "Hold", "Increase", "Close", "Reverse")
        "rationale": "string", // A brief explanation for the suggestion
        "confidence": number   // Confidence score between 0.0 and 1.0
    }}"#,
                    pos.side,
                    pos.market_mint,
                    pos.collateral_mint,
                    pos.entry_price,
                    pos.leverage,
                    pos.liquidation_price,
                    pos.pnl_after_fees_usd,
                    pos.value,
                    target_price_str, // Use processed target_price
                    stop_loss_str,    // Use processed stop_loss
                )
            })
            .collect();
        if !positions.is_empty() {
            positions_string.push_str(&positions.join(","));
        }
        positions_string.push_str("]");
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
    current_price: f64,
    symbol: &str,
    top_bids_price_amount: Vec<Vec<f64>>,
    top_asks_price_amount: Vec<Vec<f64>>,
    maybe_position_schema: String,
) -> String {
    match prediction_type {
        PredictionType::Suggestions => format!(
            r#"{{
    "summary": {{
        "vibe": "string", // Current market vibe e.g., "{symbol} Short-term 65% Bearish"
        "price": {current_price},
        "upper_bound": number, // Highest top_3_resistance
        "lower_bound": number, // Lowest top_3_support
        "technical_resistance_4h": number, // From 4h analysis
        "technical_support_4h": number, // From 4h analysis
        "top_bids_price_amount": {top_bids_price_amount:?},
        "top_asks_price_amount": {top_asks_price_amount:?},
        "detail": "string", // <500 chars, include volume and momentum insights
        "suggestion": "string" // Summary suggestion e.g., "Short {symbol} at xxx if volume confirms resistance"
    }},
    "signals": [{{
        "direction": string, // Predicted direction, long or shot
        "symbol": "{symbol}",
        "confidence": number, // Confidence about this signal: 0.0-1.0
        "current_price": {current_price},
        "entry_price": number, // Can be future price.
        "target_price": number, // >2.5% above entry, beyond first resistance or support
        "stop_loss": number, // The value should less than profit.
        "timeframe": "string", // Time in minutes or hours e.g. 5m,15m,1h,2h,3h,...
        "entry_datetime": "string", // ISO time prediction when to make a trade for this signal, Can be now or in the future date time.
        "target_datetime": "string", // ISO time prediction when to take profit.
        "rationale": "string" // Rationale about this signal e.g., "4h momentum up, bids outpace asks", "1h rejection at xxx, high ask volume"
    }}]{maybe_position_schema}
}}
"#
        ),
        PredictionType::GraphPredictions => format!(
            r#"{{
    "signals": [{{
        "direction": string, // Predicted direction, long or shot
        "symbol": "{symbol}",
        "confidence": number, // Confidence about this signal: 0.0-1.0
        "current_price": {current_price},
        "entry_price": number, // Can be future price.
        "target_price": number, // >2.5% above entry, beyond first resistance or support
        "stop_loss": number, // The value should less than profit.
        "timeframe": "string", // Time in minutes or hours e.g. 5m,15m,1h,2h,3h,...
        "entry_datetime": "string", // ISO time prediction when to make a trade for this signal, Can be now or in the future date time.
        "target_datetime": "string", // ISO time prediction when to take profit.
        "rationale": "string" // Rationale about this signal e.g., "4h momentum up, bids outpace asks", "1h rejection at xxx, high ask volume"
    }}],
    "klines": [
        [
            1741843286000,  // Open time: Timestamp in milliseconds when the K-line opens
            "123.45", // Open price: The price at the start of the time interval
            "123.45", // High price: The highest price during the time interval
            "123.45", // Low price: The lowest price during the time interval
            "123.45", // Close price: The price at the end of the time interval
            "0",   // Volume: The total trading volume during the time interval
            1741843286999,  // Close time: Timestamp in milliseconds when the K-line closes
        ]
    ]
}}
"#
        ),
    }
}
