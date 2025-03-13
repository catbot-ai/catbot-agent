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

type VisibleRange = (DateTime<Tz>, DateTime<Tz>, Vec<Kline>);

pub fn get_visible_range_and_data(
    past_data: &[Kline],
    timezone: &Tz,
    candle_width: u32,
    final_width: u32,
) -> Result<VisibleRange, Box<dyn Error>> {
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
