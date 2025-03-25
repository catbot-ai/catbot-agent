use crate::RefinedGraphPredictionResponse;
use anyhow::Result;
use reqwest::Client;

#[cfg(feature = "service_binding")]
use worker::*;

pub async fn fetch_graph_prediction_from_worker(
    req: Request,
    fetcher: &Fetcher,
    pair_symbol: &str,
    timeframe: &str, // TODO
) -> Result<RefinedGraphPredictionResponse> {
    // Construct the new path
    let new_path = format!("api/v1/predict/{pair_symbol}/{timeframe}");

    // Convert the request to HttpRequest
    let mut http_request: worker::HttpRequest = req.try_into()?;

    // Get the original URI
    let original_uri = http_request.uri();
    let scheme = original_uri.scheme_str().unwrap_or("https");
    let authority = original_uri
        .authority()
        .ok_or_else(|| worker::Error::RustError("No authority in URI".to_string()))?;

    // Construct the new URI
    let new_uri_str = format!("{}://{}/{}", scheme, authority, new_path);

    // Update the HttpRequest URI
    *http_request.uri_mut() = new_uri_str.parse()?;

    let resp = fetcher.fetch_request(http_request).await?;
    let mut cf_response: Response = resp.try_into()?;
    let response_text = cf_response.text().await?;

    let result = serde_json::from_str(&response_text)?;

    Ok(result)
}

pub async fn fetch_graph_prediction(
    api_url: &str,
    pair_symbol: &str,
    timeframe: &str, // TODO
    api_key: Option<&str>,
) -> Result<RefinedGraphPredictionResponse> {
    let client = Client::new();

    // url
    let url = format!("{api_url}/{pair_symbol}/{timeframe}");

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        binance::fetch_binance_kline_usdt, Kline, KlineValue, RefinedGraphPredictionResponse,
    };
    use chrono::{DateTime, Duration, Utc};
    use serde_json::json;

    pub async fn get_mock_graph_prediction() -> String {
        // Load real data from Binance
        let binance_pair_symbol = "SOLUSDT";
        let timeframe = "1h";
        let limit = 24;
        let candle_data = fetch_binance_kline_usdt::<Kline>(binance_pair_symbol, timeframe, limit)
            .await
            .unwrap();

        // Get current time and offset it into the future
        let now = Utc::now();
        let future_offset = Duration::minutes(1); // 1 minute into the future
        let current_time = (now + future_offset).timestamp_millis();

        // Define timezone for local time (Tokyo, +09:00)
        let tokyo = chrono_tz::Asia::Tokyo;
        let future_now_tz = (now + future_offset).with_timezone(&tokyo); // Future time in Tokyo

        // Generate current_datetime as future time
        let current_datetime = future_now_tz.to_rfc3339();

        // Generate 24 klines with the same prices, only updating time
        let mut klines = Vec::new();
        let hour_interval = 3_600_000; // 1 hour in milliseconds

        let last_candle = candle_data.last().unwrap();
        let last_open_time = last_candle.open_time;

        for i in 0..24 {
            let candle_index = i % candle_data.len();
            let historical_candle = &candle_data[candle_index];
            let open_time = last_open_time + (i + 1) as i64 * hour_interval;
            let close_time = open_time + hour_interval - 1;

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

        // Generate signal with future times
        let entry_time = current_time; // Already offset
        let target_time = (now + future_offset + Duration::hours(1)).timestamp_millis();
        let entry_time_local = (now + future_offset).to_rfc3339();
        let target_time_local = (now + future_offset + Duration::hours(1)).to_rfc3339();

        let json_data = json!({
            "current_time": current_time,
            "current_datetime": current_datetime,
            "klines": klines,
            "model_name": "gemini-2.0-flash-lite",
            "prompt_hash": "7b73af1c95c40c59b856d6cfd5b7f31d",
            "signals": [{
                "confidence": 0.7,
                "current_price": last_candle.close_price.parse::<f64>().unwrap(),
                "direction": "long",
                "entry_time": entry_time,
                "entry_time_local": entry_time_local,
                "entry_price": last_candle.close_price.parse::<f64>().unwrap(),
                "rationale": "Based on the 1h price history, SOL is showing signs of a potential bullish reversal. Stochastic RSI is currently below 20, indicating oversold conditions. Recent price action shows strong support. 1h volume is increasing.",
                "stop_loss": last_candle.close_price.parse::<f64>().unwrap() * 0.97,
                "pair_symbol": "SOL_USDT",
                "target_time": target_time,
                "target_time_local": target_time_local,
                "target_price": last_candle.close_price.parse::<f64>().unwrap() * 1.03,
                "timeframe": "1h"
            }]
        });

        serde_json::to_string_pretty(&json_data).unwrap()
    }

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

        let _parsed =
            serde_json::from_str::<RefinedGraphPredictionResponse>(&mock_json.clone()).unwrap();

        // Verify top-level fields
        let current_time = json_value["current_time"].as_i64().unwrap();
        let current_datetime = json_value["current_datetime"].as_str().unwrap();

        // Check that times are in the future
        let now = Utc::now();
        let current_time = DateTime::from_timestamp_millis(current_time).unwrap();
        let current_datetime = DateTime::parse_from_rfc3339(current_datetime).unwrap();
        assert!(current_time > now);
        assert!(current_datetime > now.with_timezone(&chrono_tz::Asia::Tokyo));

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
        let entry_time = signal["entry_time"].as_i64().unwrap();
        let target_time = signal["target_time"].as_i64().unwrap();
        let entry_dt = DateTime::from_timestamp(entry_time / 1000, 0).unwrap();
        let target_dt = DateTime::from_timestamp(target_time / 1000, 0).unwrap();
        assert!(entry_dt > now);
        assert_eq!(target_dt - entry_dt, Duration::hours(1));
    }
}
