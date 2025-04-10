use m4rs::Candlestick;

use crate::Kline;
pub fn kline_to_m4rs_candlestick(kline: &Kline) -> Candlestick {
    Candlestick::new(
        kline.open_time as u64,
        kline.open_price.parse::<f64>().unwrap(),
        kline.high_price.parse::<f64>().unwrap(),
        kline.low_price.parse::<f64>().unwrap(),
        kline.close_price.parse::<f64>().unwrap(),
        kline.volume.parse::<f64>().unwrap(),
    )
}
