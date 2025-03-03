use anyhow::{Result, *};
use regex::Regex;
use serde::de::DeserializeOwned;
use std::result::Result::Ok;

pub fn try_parse_json_with_trailing_comma_removal<T: DeserializeOwned>(
    json_string: &str,
) -> Result<T> {
    match serde_json::from_str(json_string) {
        Ok(parsed) => Ok(parsed),
        Err(original_error) => {
            let cleaned_json_string = fix_trailing_commas(json_string);
            serde_json::from_str(&cleaned_json_string).map_err(|e| {
                anyhow!(
                    "Failed to parse cleaned JSON: {}. Original error: {}",
                    e,
                    original_error
                )
            })
        }
    }
}

fn fix_trailing_commas(json_str: &str) -> String {
    // Regex pattern to match a comma followed by optional whitespace and a closing bracket/brace
    let re = Regex::new(r#",(\s*[\]}])"#).unwrap();

    // Replace ",]" or ",}" (with optional whitespace) with just "]" or "}"
    re.replace_all(json_str, "$1").to_string()
}

#[test]
fn test_fix_trailing_comma_and_deserialize() {
    use common::PredictionOutput;

    let raw_json = r#"{
        "summary": {
            "title": "SOL Short-term Bearish",
            "price": 164.62,
            "upper_bound": 171.36,
            "lower_bound": 163.9,
            "technical_resistance_4h": 170.12,
            "technical_support_4h": 167.8,
            "top_bids_price_amount": [
                [164.0, 21198.086000000003],
                [163.0, 10543.815999999999],
                [162.0, 4982.048999999999],
                [161.0, 4122.828000000002],
                [160.0, 9694.809999999998]
            ],
            "top_asks_price_amount": [
                [165.0, 22694.669],
                [166.0, 12218.673000000003],
                [167.0, 6445.126000000002],
                [168.0, 3369.6710000000003],
                [169.0, 2392.2509999999997]
            ],
            "detail": "1m and 5m chart shows some bearish momentum. The 1h chart has broken support at 169.5. Selling volume has been increasing over last 1h. Order book ask volume is higher than bid volume.",
            "suggestion": "Hold short position. Consider reversing the position if price breaks above 170.",
            "vibe": "Bearish 70%"
        },
        "signals": [
            {
                "side": "short",
                "symbol": "SOL",
                "confidence": 0.7,
                "current_price": 164.62,
                "entry_price": 164.62,
                "target_price": 159.72,
                "stop_loss": 167.82,
                "timeframe": "1h",
                "entry_datetime": "2025-03-03T13:16:33Z",
                "target_datetime": "2025-03-03T16:16:33Z",
                "rationale": "1m and 5m price is moving down after breaking support. The 1h chart volume has been increasing.",
            }
        ],
        "positions": [
            {
                "side": "short",
                "market_mint": "So11111111111111111111111111111111111111112",
                "collateral_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "entry_price": 163.83,
                "leverage": 9.99,
                "liquidation_price": 179.77,
                "pnl_after_fees_usd": -29.61,
                "value": 470.34,
                "target_price": 162.0,
                "stop_loss": 167.82,
                "suggestion": "Hold short position. Consider increasing position at 164.0 with stop loss at 167.82 and taking profit at 159.72.",
                "new_target_price": 159.72,
                "new_stop_loss": 167.82,
                "confidence": 0.7,
            }
        ]
    }"#;

    let fixed_json = fix_trailing_commas(raw_json);

    let result = serde_json::from_str::<PredictionOutput>(&fixed_json);
    assert!(result.is_ok());

    if let Ok(response) = result {
        assert_eq!(response.summary.title, "SOL Short-term Bearish");
        assert_eq!(response.positions.unwrap().len(), 1);
    }
}
