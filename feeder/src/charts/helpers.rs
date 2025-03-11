use chrono::{DateTime, Duration};
use chrono_tz::Tz;
use common::Kline;
use m4rs::Candlestick as M4rsCandlestick;
use std::error::Error;

pub fn parse_kline_time(timestamp: i64, tz: &Tz) -> DateTime<Tz> {
    DateTime::from_timestamp(timestamp / 1000, 0)
        .unwrap()
        .with_timezone(tz)
}

pub fn kline_to_m4rs_candlestick(k: &Kline) -> M4rsCandlestick {
    M4rsCandlestick::new(
        k.open_time as u64,
        k.open_price.parse::<f64>().unwrap(),
        k.high_price.parse::<f64>().unwrap(),
        k.low_price.parse::<f64>().unwrap(),
        k.close_price.parse::<f64>().unwrap(),
        k.volume.parse::<f64>().unwrap(),
    )
}

pub fn parse_timeframe_duration(timeframe: &str) -> Duration {
    let (value, unit) = timeframe.split_at(timeframe.len() - 1);
    let value = value.parse::<i64>().unwrap();
    match unit {
        "m" => Duration::minutes(value),
        "h" => Duration::hours(value),
        "d" => Duration::days(value),
        _ => panic!("Unsupported timeframe unit"),
    }
}

pub fn get_visible_range_and_data(
    past_data: &[Kline],
    timezone: &Tz,
    candle_width: u32,
    final_width: u32,
) -> Result<(DateTime<Tz>, DateTime<Tz>, Vec<Kline>), Box<dyn Error>> {
    let total_candles = past_data.len();
    if total_candles == 0 {
        return Err("No candle data available".into());
    }

    let visible_candles = (final_width / candle_width) as usize;
    let start_index = total_candles.saturating_sub(visible_candles);

    let first_visible_time = parse_kline_time(past_data[start_index].open_time, timezone);
    let last_visible_time = parse_kline_time(past_data[total_candles - 1].open_time, timezone);

    let visible_data: Vec<Kline> = past_data
        .iter()
        .filter(|k| {
            let time = parse_kline_time(k.open_time, timezone);
            time >= first_visible_time && time <= last_visible_time
        })
        .cloned()
        .collect();

    Ok((first_visible_time, last_visible_time, visible_data))
}

pub fn calculate_sma(close_prices: &[f32], period: usize) -> Result<Vec<f32>, Box<dyn Error>> {
    if period == 0 {
        return Err("Period must be greater than 0".into());
    }
    if close_prices.len() < period {
        return Err(format!("Not enough data: need at least {} prices", period).into());
    }

    let mut sma = Vec::with_capacity(close_prices.len());
    for i in 0..close_prices.len() {
        if i < period - 1 {
            sma.push(0.0); // Not enough data yet
        } else {
            let sum: f32 = close_prices[i - (period - 1)..=i].iter().sum();
            sma.push(sum / period as f32);
        }
    }
    Ok(sma)
}

pub fn extract_signals(
    klines: &[Kline],
    ma_short: usize,
    ma_long: usize,
    profit_percent: f32,
) -> Result<(Vec<(i64, f32, f32)>, Vec<(i64, f32, f32)>), Box<dyn Error>> {
    if ma_short >= ma_long {
        return Err("ma_short must be less than ma_long".into());
    }
    if klines.len() < ma_long {
        return Err(format!("Not enough klines: need at least {} klines", ma_long).into());
    }

    let close_prices: Vec<f32> = klines
        .iter()
        .map(|k| {
            k.close_price
                .parse::<f32>()
                .unwrap_or_else(|_| panic!("Invalid close price"))
        })
        .collect();
    let sma_short = calculate_sma(&close_prices, ma_short)?;
    let sma_long = calculate_sma(&close_prices, ma_long)?;

    let mut long_signals = Vec::new();
    let mut short_signals = Vec::new();

    for i in 1..klines.len() {
        if sma_short[i - 1] <= sma_long[i - 1] && sma_short[i] > sma_long[i] {
            // Buy signal (long position)
            let entry_price = close_prices[i];
            let target_price = entry_price * (1.0 + profit_percent);
            long_signals.push((klines[i].open_time, entry_price, target_price));
        } else if sma_short[i - 1] >= sma_long[i - 1] && sma_short[i] < sma_long[i] {
            // Sell signal (short position)
            let entry_price = close_prices[i];
            let target_price = entry_price * (1.0 - profit_percent);
            short_signals.push((klines[i].open_time, entry_price, target_price));
        }
    }

    Ok((long_signals, short_signals))
}
