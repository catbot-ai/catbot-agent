use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Kline {
    pub open_time: i64,
    pub open_price: String,
    pub high_price: String,
    pub low_price: String,
    pub close_price: String,
    pub volume: String,
    pub close_time: i64,
    #[serde(default)]
    pub quote_asset_volume: String,
    #[serde(default)]
    pub number_of_trades: i64,
    #[serde(default)]
    pub taker_buy_base_asset_volume: String,
    #[serde(default)]
    pub taker_buy_quote_asset_volume: String,
    #[serde(default)]
    pub ignore: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConciseKline {
    pub close_time: i64,
    #[serde(serialize_with = "serialize_price")]
    pub high: f64,
    #[serde(serialize_with = "serialize_price")]
    pub low: f64,
    #[serde(serialize_with = "serialize_price")]
    pub close: f64,
    #[serde(serialize_with = "serialize_volume")]
    pub volume: f64,
}

fn serialize_price<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = if *value >= 1.0 {
        (*value * 100.0).round() / 100.0 // 2 decimals
    } else {
        (*value * 10000000.0).round() / 10000000.0 // 7 decimals
    };
    serializer.serialize_f64(rounded)
}

// Same for volume, adjust decimals
fn serialize_volume<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = if *value >= 1.0 {
        (*value * 1000.0).round() / 1000.0 // 3 decimals
    } else {
        (*value * 10000000.0).round() / 10000000.0 // 7 decimals
    };
    serializer.serialize_f64(rounded)
}

impl From<Kline> for ConciseKline {
    fn from(kline: Kline) -> Self {
        ConciseKline {
            close_time: kline.open_time,
            high: kline.high_price.parse().unwrap_or(0.0),
            low: kline.low_price.parse().unwrap_or(0.0),
            close: kline.close_price.parse().unwrap_or(0.0),
            volume: kline.volume.parse().unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OrderBook {
    pub last_update_id: i64,
    pub bids: Vec<Vec<String>>,
    pub asks: Vec<Vec<String>>,
}
