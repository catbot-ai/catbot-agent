use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Deserialize, Serialize)]
pub struct Kline {
    pub open_time: i64,
    pub open_price: String,
    pub high_price: String,
    pub low_price: String,
    pub close_price: String,
    pub volume: String,
    pub close_time: i64,
    pub quote_asset_volume: String,
    pub number_of_trades: i64,
    pub taker_buy_base_asset_volume: String,
    pub taker_buy_quote_asset_volume: String,
    pub ignore: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConciseKline {
    pub close_time: i64,
    #[serde(serialize_with = "remove_trailing_zeros")]
    pub high_price: String,
    #[serde(serialize_with = "remove_trailing_zeros")]
    pub low_price: String,
    #[serde(serialize_with = "remove_trailing_zeros")]
    pub close_price: String,
    #[serde(serialize_with = "remove_trailing_zeros")]
    pub volume: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClosePriceKline {
    pub open_time: i64,
    #[serde(serialize_with = "remove_trailing_zeros")]
    pub close_price: String,
    #[serde(serialize_with = "remove_trailing_zeros")]
    pub volume: String,
}

fn remove_trailing_zeros<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut s = value.trim_end_matches('0').to_string();
    if s.ends_with('.') {
        s.pop();
    }
    serializer.serialize_str(&s)
}

impl From<Kline> for ConciseKline {
    fn from(kline: Kline) -> Self {
        ConciseKline {
            close_time: kline.open_time,
            high_price: kline.high_price.clone(),
            low_price: kline.low_price.clone(),
            close_price: kline.close_price.clone(),
            volume: kline.volume.clone(),
        }
    }
}

impl From<Kline> for ClosePriceKline {
    fn from(kline: Kline) -> Self {
        ClosePriceKline {
            open_time: kline.open_time,
            close_price: kline.close_price.clone(),
            volume: kline.volume.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBook {
    pub last_update_id: i64,
    pub bids: Vec<Vec<String>>,
    pub asks: Vec<Vec<String>>,
}
