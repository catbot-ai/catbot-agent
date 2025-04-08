use m4rs::Candlestick;

use crate::Kline;
pub fn kline_to_m4rs_candlestick(k: &Kline) -> Candlestick {
    Candlestick::new(
        k.open_time as u64,
        k.open_price.parse::<f64>().unwrap(),
        k.high_price.parse::<f64>().unwrap(),
        k.low_price.parse::<f64>().unwrap(),
        k.close_price.parse::<f64>().unwrap(),
        k.volume.parse::<f64>().unwrap(),
    )
}
