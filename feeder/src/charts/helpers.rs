use chrono::{DateTime, Duration};
use chrono_tz::Tz;
use common::Kline;
use std::error::Error;

pub fn parse_kline_time(timestamp: i64, tz: &Tz) -> DateTime<Tz> {
    DateTime::from_timestamp(timestamp / 1000, 0)
        .unwrap()
        .with_timezone(tz)
}

pub fn parse_interval_duration(interval: &str) -> Duration {
    let (value, unit) = interval.split_at(interval.len() - 1);
    let value = value.parse::<i64>().unwrap();
    match unit {
        "m" => Duration::minutes(value),
        "h" => Duration::hours(value),
        "d" => Duration::days(value),
        _ => panic!("Unsupported interval unit"),
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

    let visible_candles = (final_width as f32 / candle_width as f32).ceil() as usize;
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

pub fn format_short_number(num: i64) -> String {
    if num < 1000 {
        return num.to_string();
    }

    let float_num = num as f64;
    if num < 1_000_000 {
        let result = float_num / 1000.0;
        format!("{:.2}K", result)
    } else {
        let result = float_num / 1_000_000.0;
        format!("{:.2}M", result)
    }
}
