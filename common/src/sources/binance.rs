use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value as JsonValue;

use crate::{Kline, OrderBook};

const BINANCE_API_URL: &str = "https://data-api.binance.vision/api/v3";

pub fn get_token_and_pair_symbol_usdt(pair_symbol: &str) -> (String, String) {
    let token_symbol = pair_symbol.split("_").next().unwrap();
    let token_symbol = token_symbol.split("USD").next().unwrap();

    // We need USDT orderbook
    let binance_pair_symbol = format!("{token_symbol}USDT");
    (token_symbol.to_string(), binance_pair_symbol)
}

pub async fn fetch_binance_kline_usdt<T>(
    pair_symbol: &str,
    interval: &str,
    limit: i32,
) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned + Send + std::convert::From<Kline>,
{
    let (_, binance_pair_symbol) = get_token_and_pair_symbol_usdt(pair_symbol);

    let client = Client::new();
    // let current_time = Utc::now().timestamp_millis();
    // https://adversely-amazing-wildcat.edgecompute.app/?url=https://data-api.binance.vision/api/v3/uiKlines?limit=1&symbol=SOLUSDT&interval=1s

    let url = format!(
        "https://adversely-amazing-wildcat.edgecompute.app/?url={BINANCE_API_URL}/uiKlines?limit={limit}&symbol={binance_pair_symbol}&interval={interval}"
    );

    println!("Fetching data from: {url}");

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

pub async fn fetch_orderbook_depth_usdt(pair_symbol: &str, limit: i32) -> Result<OrderBook> {
    // We need USDT orderbook
    let (_, binance_pair_symbol) = get_token_and_pair_symbol_usdt(pair_symbol);

    let client = Client::new();
    // https://adversely-amazing-wildcat.edgecompute.app/?url=https://api.binance.com/api/v3/depth?symbol=SOLUSDT&limit=1
    let url = format!(
        "https://adversely-amazing-wildcat.edgecompute.app/?url={BINANCE_API_URL}/depth?symbol={binance_pair_symbol}&limit={limit}"
    );
    let response = client.get(&url).send().await?;
    let orderbook_data: OrderBook = response.json().await?;

    Ok(orderbook_data)
}

/// Fetches Binance Kline data for a given pair symbol, interval, and limit, and returns it as a CSV string.
///
/// # Arguments
/// * `pair_symbol` - The trading pair symbol (e.g., "SOLUSDT").
/// * `interval` - The time interval (e.g., "1h", "1d").
/// * `limit` - The number of Kline data points to fetch.
///
/// # Returns
/// A `Result<String>` containing the CSV data with the header `open_time,open,high,low,close,volume,close_time`.
///
/// # Errors
/// Returns an `anyhow::Error` if fetching or processing the data fails.
pub async fn fetch_binance_kline_usdt_csv(
    pair_symbol: &str,
    interval: &str,
    limit: i32,
) -> Result<String> {
    // We need USDT orderbook
    let (_, binance_pair_symbol) = get_token_and_pair_symbol_usdt(pair_symbol);

    // Fetch raw Kline data
    let kline_data: Vec<Kline> = fetch_binance_kline_usdt::<Kline>(&binance_pair_symbol, interval, limit)
        .await
        .with_context(|| format!("Failed to fetch Kline data for {binance_pair_symbol} with interval {interval} and limit {limit}"))?;

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

pub fn klines_to_csv(klines: &[Kline]) -> anyhow::Result<String> {
    let mut csv_string = String::new();
    // Add header
    csv_string.push_str("open_time,open,high,low,close,volume,close_time\n");

    // Add each Kline record
    for kline in klines {
        let values = kline.to_array()?;
        let mut is_first = true;
        for value in values {
            if !is_first {
                csv_string.push(',');
            }
            // Handle potential non-number values gracefully if Kline::to_array can return them
            match value {
                JsonValue::Number(n) => csv_string.push_str(&n.to_string()),
                JsonValue::String(s) => csv_string.push_str(&s), // Or handle as error if only numbers expected
                _ => csv_string.push_str(""), // Or some placeholder/error indication
            }
            is_first = false;
        }
        csv_string.push('\n');
    }
    Ok(csv_string)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_get_token_and_pair_symbol() {
        let pair_symbols = ["SOL", "SOLUSDC", "SOL_USDC", "SOLUSDT", "SOL_USDT"];
        for pair_symbol in pair_symbols {
            let (token_symbol, binance_pair_symbol) = get_token_and_pair_symbol_usdt(pair_symbol);
            assert_eq!(token_symbol, "SOL");
            assert_eq!(binance_pair_symbol, "SOLUSDT");
        }
    }

    #[tokio::test]
    async fn test() {
        use crate::ConciseKline;

        let pair_symbol = "SOL_USDT";
        let interval = "1h";

        println!("Fetcher started for symbol: {pair_symbol}, interval: {interval}");

        let kline_data = fetch_binance_kline_usdt::<Kline>(pair_symbol, interval, 1)
            .await
            .unwrap();
        println!("Fetched {} Kline data points", kline_data.len()); // Log data points fetched

        let kline_data = fetch_binance_kline_usdt::<ConciseKline>(pair_symbol, interval, 1)
            .await
            .unwrap();
        println!("Fetched {} ConciseKline data points", kline_data.len()); // Log data points fetched

        assert!(!kline_data.is_empty());
    }
}
