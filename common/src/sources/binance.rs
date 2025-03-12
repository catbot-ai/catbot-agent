use anyhow::{anyhow, Context, Result};

use reqwest::Client;

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
    let pair_symbol = pair_symbol.replace("_", "");
    let client = Client::new();
    // let current_time = Utc::now().timestamp_millis();
    // https://adversely-amazing-wildcat.edgecompute.app/?url=https://data-api.binance.vision/api/v3/uiKlines?limit=1&symbol=SOLUSDT&interval=1s

    let url = format!(
        "https://adversely-amazing-wildcat.edgecompute.app/?url={BINANCE_API_URL}/uiKlines?limit={}&symbol={}&interval={}",
        limit, pair_symbol, interval
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
    let pair_symbol = pair_symbol.replace("_", "");
    println!("fetch_orderbook_depth: {}", pair_symbol);
    let client = Client::new();
    // https://adversely-amazing-wildcat.edgecompute.app/?url=https://api.binance.com/api/v3/depth?symbol=SOLUSDT&limit=1
    let url = format!(
        "https://adversely-amazing-wildcat.edgecompute.app/?url={BINANCE_API_URL}/depth?symbol={}&limit={}",
        pair_symbol, limit
    );
    let response = client.get(&url).send().await?;
    let orderbook_data: OrderBook = response.json().await?;

    Ok(orderbook_data)
}

#[cfg(test)]
#[tokio::test]
async fn test() {
    use crate::ConciseKline;

    let pair_symbol = "SOL_USDT";
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
