use anyhow::{anyhow, Context, Result};

use reqwest::Client;
use serde_json::Value as JsonValue;

use crate::{Kline, OrderBook};

const BINANCE_API_URL: &str = "https://data-api.binance.vision/api/v3";

pub async fn fetch_binance_kline_data<T>(
    pair_symbol: &str,
    interval: &str,
    limit: i32,
) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned + Send + std::convert::From<Kline>,
{
    let binance_pair_symbol = pair_symbol.replace("_", "");
    let client = Client::new();
    // let current_time = Utc::now().timestamp_millis();
    // https://adversely-amazing-wildcat.edgecompute.app/?url=https://data-api.binance.vision/api/v3/uiKlines?limit=1&symbol=SOLUSDC&interval=1s

    let url = format!(
        "https://adversely-amazing-wildcat.edgecompute.app/?url={BINANCE_API_URL}/uiKlines?limit={}&symbol={}&interval={}",
        limit, binance_pair_symbol, interval
    );

    println!("Fetching data from: {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to Binance API")?;

    if !response.status().is_success() {
        return Err(anyhow!("Binance API error: {:?}", response.status()));
    }

    let kline_data: Vec<Kline> = response
        .json()
        .await
        .context("Failed to parse JSON response from Binance API")?;

    let concise_kline_data: Vec<T> = kline_data.into_iter().map(|kline| kline.into()).collect();

    Ok(concise_kline_data)
}

pub async fn fetch_orderbook_depth(pair_symbol: &str, limit: i32) -> Result<OrderBook> {
    let binance_pair_symbol = pair_symbol.replace("_", "");
    println!("fetch_orderbook_depth: {}", binance_pair_symbol);
    let client = Client::new();
    // https://adversely-amazing-wildcat.edgecompute.app/?url=https://api.binance.com/api/v3/depth?symbol=SOLUSDC&limit=1
    let url = format!(
        "https://adversely-amazing-wildcat.edgecompute.app/?url={BINANCE_API_URL}/depth?symbol={}&limit={}",
        binance_pair_symbol, limit
    );
    let response = client.get(&url).send().await?;
    let orderbook_data: OrderBook = response.json().await?;

    Ok(orderbook_data)
}

/// Fetches Binance Kline data for a given pair symbol, interval, and limit, and returns it as a CSV string.
///
/// # Arguments
/// * `pair_symbol` - The trading pair symbol (e.g., "SOLUSDC").
/// * `interval` - The time interval (e.g., "1h", "1d").
/// * `limit` - The number of Kline data points to fetch.
///
/// # Returns
/// A `Result<String>` containing the CSV data with the header `open_time,open,high,low,close,volume,close_time`.
///
/// # Errors
/// Returns an `anyhow::Error` if fetching or processing the data fails.
pub async fn fetch_binance_kline_csv(
    pair_symbol: &str,
    interval: &str,
    limit: i32,
) -> Result<String> {
    // Fetch raw Kline data
    let kline_data: Vec<Kline> = fetch_binance_kline_data::<Kline>(pair_symbol, interval, limit)
        .await
        .with_context(|| format!("Failed to fetch Kline data for {pair_symbol} with interval {interval} and limit {limit}"))?;

    // Build CSV string manually
    let mut csv_string = String::new();

    // Add header
    csv_string.push_str("open_time,open,high,low,close,volume,close_time\n");

    // Add each Kline record
    for kline in kline_data {
        let values = kline.to_array()?;
        let mut is_first = true;
        for value in values {
            if !is_first {
                csv_string.push(',');
            }
            if let JsonValue::Number(n) = value {
                csv_string.push_str(&n.to_string());
            }
            is_first = false;
        }
        csv_string.push('\n');
    }

    Ok(csv_string)
}

#[cfg(test)]
#[tokio::test]
async fn test() {
    use crate::ConciseKline;

    let pair_symbol = "SOL_USDC";
    let interval = "1h";

    println!(
        "Fetcher started for symbol: {}, interval: {}",
        pair_symbol, interval
    );

    let kline_data = fetch_binance_kline_data::<Kline>(pair_symbol, interval, 1)
        .await
        .unwrap();
    println!("Fetched {} Kline data points", kline_data.len()); // Log data points fetched

    let kline_data = fetch_binance_kline_data::<ConciseKline>(pair_symbol, interval, 1)
        .await
        .unwrap();
    println!("Fetched {} ConciseKline data points", kline_data.len()); // Log data points fetched

    assert!(!kline_data.is_empty());
}
