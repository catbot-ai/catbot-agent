use anyhow::Result;
use chrono::{Duration, Utc};
use reqwest::Client;
use serde_json::json;

use crate::{Kline, KlineValue, RefinedGraphPredictionResponse};

use super::binance::fetch_binance_kline_data;

pub async fn fetch_graph_prediction(
    api_url: &str,
    pair_symbol: &str,
    interval: &str, // TODO
    api_key: Option<&str>,
) -> Result<RefinedGraphPredictionResponse> {
    let client = Client::new();

    // url
    let url = format!("{api_url}/{pair_symbol}/{interval}");
    println!("{url}");

    // Build the request
    let mut request = client.get(url);

    // Add API key to headers if provided
    if let Some(key) = api_key {
        request = request.header("Authorization", format!("Bearer {}", key));
    }

    // Send the request and get the response
    let response = request
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;

    // Check if the response status is successful
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Request failed with status: {}",
            response.status()
        ));
    }

    // Deserialize the response body into RefinedGraphPredictionResponse
    let prediction = response
        .json::<RefinedGraphPredictionResponse>()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to deserialize response: {}", e))?;

    Ok(prediction)
}

pub async fn get_mock_graph_prediction() -> String {
    // Load real data from Binance
    let binance_pair_symbol = "SOLUSDT";
    let timeframe = "1h";
    let limit = 24; // Fetch historical candles
    let candle_data = fetch_binance_kline_data::<Kline>(binance_pair_symbol, timeframe, limit)
        .await
        .unwrap();

    // Get current time
    let current_time = Utc::now();

    // Define timezone for local time (Tokyo, +09:00)
    let tokyo = chrono_tz::Asia::Tokyo;
    let current_time_local = current_time.with_timezone(&tokyo);

    // Generate timestamp (milliseconds since Unix epoch)
    let timestamp = current_time.timestamp_millis();

    // Generate current_datetime and current_datetime_local
    let current_datetime = current_time.to_rfc3339();
    let current_datetime_local = current_time_local.to_rfc3339();

    // Generate 24 klines with the same prices, only updating time
    let mut klines = Vec::new();
    let hour_interval = 3_600_000; // 1 hour in milliseconds

    // Start from the last real candle
    let last_candle = candle_data.last().unwrap();
    let last_open_time = last_candle.open_time;

    // Generate 24 future candles by cycling through the historical candles
    for i in 0..24 {
        // Cycle through the historical candles (0 to 9)
        let candle_index = i % candle_data.len();
        let historical_candle = &candle_data[candle_index];

        // Update the timestamps for the future
        let open_time = last_open_time + (i + 1) as i64 * hour_interval;
        let close_time = open_time + hour_interval - 1;

        // Use the historical candle's prices and volume
        let kline_values = vec![
            KlineValue::Int64(open_time),
            KlineValue::String(historical_candle.open_price.clone()),
            KlineValue::String(historical_candle.high_price.clone()),
            KlineValue::String(historical_candle.low_price.clone()),
            KlineValue::String(historical_candle.close_price.clone()),
            KlineValue::String(historical_candle.volume.clone()),
            KlineValue::Int64(close_time),
        ];
        klines.push(kline_values);
    }

    println!("{klines:#?}");

    // Generate signal with current time and 1 hour later
    let entry_datetime = current_time.to_rfc3339();
    let target_datetime = (current_time + Duration::hours(1)).to_rfc3339();
    let entry_datetime_local = current_time_local.to_rfc3339();
    let target_datetime_local = (current_time_local + Duration::hours(1)).to_rfc3339();

    // Construct the JSON object
    let json_data = json!({
        "current_datetime": current_datetime,
        "current_datetime_local": current_datetime_local,
        "klines": klines,
        "model_name": "gemini-2.0-flash-lite",
        "prompt_hash": "7b73af1c95c40c59b856d6cfd5b7f31d",
        "signals": [{
            "confidence": 0.7,
            "current_price": last_candle.close_price.parse::<f64>().unwrap(), // Use the last close price as the current price
            "direction": "long",
            "entry_datetime": entry_datetime,
            "entry_datetime_local": entry_datetime_local,
            "entry_price": last_candle.close_price.parse::<f64>().unwrap(), // Use the last close price as the entry price
            "rationale": "Based on the 1h price history, SOL is showing signs of a potential bullish reversal. Stochastic RSI is currently below 20, indicating oversold conditions. Recent price action shows strong support. 1h volume is increasing.",
            "stop_loss": last_candle.close_price.parse::<f64>().unwrap() * 0.97, // 3% below entry price
            "symbol": "SOL",
            "target_datetime": target_datetime,
            "target_datetime_local": target_datetime_local,
            "target_price": last_candle.close_price.parse::<f64>().unwrap() * 1.03, // 3% above entry price
            "timeframe": "1h"
        }],
        "timestamp": timestamp
    });

    // Serialize to pretty-printed JSON string
    serde_json::to_string_pretty(&json_data).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    #[tokio::test]
    async fn test_fetch_graph_prediction() {
        dotenvy::from_filename(".env").expect("No .env file");
        let api_url = std::env::var("PREDICTION_API_URL").expect("PREDICTION_API_URL must be set");

        let pair_symbol = "SOL_USDT";
        let timeframe = "1h";

        let prediction = fetch_graph_prediction(&api_url, pair_symbol, timeframe, None)
            .await
            .unwrap();

        println!("{prediction:#?}");
    }

    #[tokio::test]
    async fn test_get_mock_graph_prediction() {
        // Get the mock prediction
        let mock_json = get_mock_graph_prediction().await;

        // Parse the JSON to verify
        let json_value: serde_json::Value = serde_json::from_str(&mock_json.clone()).unwrap();

        let parsed =
            serde_json::from_str::<RefinedGraphPredictionResponse>(&mock_json.clone()).unwrap();

        println!("{parsed:#?}");

        // Verify top-level fields
        let current_datetime = json_value["current_datetime"].as_str().unwrap();
        let current_datetime_local = json_value["current_datetime_local"].as_str().unwrap();
        let timestamp = json_value["timestamp"].as_i64().unwrap();

        // Check that times are in the future
        let now = Utc::now();
        let current_dt = DateTime::parse_from_rfc3339(current_datetime).unwrap();
        let current_dt_local = DateTime::parse_from_rfc3339(current_datetime_local).unwrap();
        assert!(current_dt > now);
        assert!(current_dt_local > now.with_timezone(&chrono_tz::Asia::Tokyo));
        assert!(timestamp > now.timestamp_millis());

        // Verify klines
        let klines = json_value["klines"].as_array().unwrap();
        assert_eq!(klines.len(), 24); // 24 candles
        for i in 0..23 {
            let current_kline = &klines[i];
            let next_kline = &klines[i + 1];
            let current_time = current_kline[0].as_i64().unwrap();
            let next_time = next_kline[0].as_i64().unwrap();
            assert_eq!(next_time - current_time, 3_600_000); // 1-hour interval
            assert!(current_time > now.timestamp_millis());
        }

        // Verify signal times
        let signals = json_value["signals"].as_array().unwrap();
        let signal = &signals[0];
        let entry_datetime = signal["entry_datetime"].as_str().unwrap();
        let target_datetime = signal["target_datetime"].as_str().unwrap();
        let entry_dt = DateTime::parse_from_rfc3339(entry_datetime).unwrap();
        let target_dt = DateTime::parse_from_rfc3339(target_datetime).unwrap();
        assert!(entry_dt > now);
        assert_eq!(target_dt - entry_dt, Duration::hours(1));
    }
}
