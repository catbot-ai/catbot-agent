use common::OrderBook;
use std::collections::BTreeMap;
use strum::{Display, EnumString};

#[derive(Debug, EnumString, Display)]
pub enum FractionalPart {
    #[strum(serialize = "0.1")]
    OneTenth,
    #[strum(serialize = "1")]
    One,
    #[strum(serialize = "10")]
    Ten,
    #[strum(serialize = "100")]
    Hundred,
}

pub fn group_by_fractional_part(
    orderbook_data: &OrderBook,
    fractional_part: FractionalPart,
) -> (BTreeMap<String, f64>, BTreeMap<String, f64>) {
    let mut grouped_bids: BTreeMap<String, f64> = BTreeMap::new();
    let mut grouped_asks: BTreeMap<String, f64> = BTreeMap::new();

    let multiplier = match fractional_part {
        FractionalPart::OneTenth => 10.0,
        FractionalPart::One => 1.0,
        FractionalPart::Ten => 0.1,
        FractionalPart::Hundred => 0.01,
    };

    for bid in &orderbook_data.bids {
        if bid.len() == 2 {
            if let (Ok(price_str), Ok(amount_str)) = (bid[0].parse::<f64>(), bid[1].parse::<f64>())
            {
                let price = (price_str * multiplier).floor() / multiplier;
                let price_str = format!("{:.0}", price); // Format to avoid floating point issues in keys
                *grouped_bids.entry(price_str).or_insert(0.0) += amount_str;
            }
        }
    }

    for ask in &orderbook_data.asks {
        if ask.len() == 2 {
            if let (Ok(price_str), Ok(amount_str)) = (ask[0].parse::<f64>(), ask[1].parse::<f64>())
            {
                let price = (price_str * multiplier).ceil() / multiplier;
                let price_str = format!("{:.0}", price); // Format to avoid floating point issues in keys
                *grouped_asks.entry(price_str).or_insert(0.0) += amount_str;
            }
        }
    }

    println!("Grouped Asks: {:?}", grouped_asks);
    println!("Grouped Bids: {:?}", grouped_bids);

    (grouped_bids, grouped_asks)
}

type PriceAmountVec = Vec<(String, f64)>;

pub fn top_n_support_resistance(
    grouped_data: &BTreeMap<String, f64>,
    n: usize,
) -> (PriceAmountVec, PriceAmountVec) {
    let mut sorted_data: Vec<(&String, &f64)> = grouped_data.iter().collect();

    sorted_data.sort_by_key(|&(k, _)| k);

    let top_n = sorted_data
        .iter()
        .take(n)
        .map(|(k, v)| (k.to_string(), **v))
        .collect();

    let bottom_n = sorted_data
        .iter()
        .rev()
        .take(n)
        .map(|(k, v)| (k.to_string(), **v))
        .collect();

    println!("Top N: {:?}", top_n);
    println!("Bottom N: {:?}", bottom_n);

    (top_n, bottom_n)
}

pub fn to_csv(data: &[(String, f64)]) -> String {
    let mut csv = String::from("price,cumulative_amount\n");
    for (price_str, amount) in data {
        // Parse the price string back to f64 to format it.
        if let Ok(price) = price_str.parse::<f64>() {
            csv.push_str(&format!("{:.0},{:.3}\n", price, amount)); // Format price to 1 decimal place
        } else {
            // Handle parsing errors if necessary (e.g., log a warning).
            eprintln!("Error parsing price: {}", price_str);
        }
    }
    csv
}

#[cfg(test)]
#[tokio::test]
async fn test_group_and_top_n() {
    // let orderbook_json = r#"{"lastUpdateId":18560646066,"bids":[["170.02000000","204.47900000"],["170.01000000","150.14900000"],["170.00000000","86.51000000"],["169.99000000","104.08900000"],["169.98000000","168.26600000"],["169.97000000","102.02100000"],["169.96000000","189.04000000"],["169.95000000","190.76100000"],["169.94000000","308.73800000"],["169.93000000","224.72800000"]],"asks":[["170.03000000","12.03800000"],["170.04000000","3.84100000"],["170.05000000","34.67200000"],["170.06000000","90.68600000"],["170.07000000","200.38200000"],["170.08000000","98.31900000"],["170.09000000","102.28700000"],["170.10000000","196.39600000"],["170.11000000","191.37100000"],["170.12000000","169.14700000"]]}"#;
    // let orderbook: OrderBook = serde_json::from_str(orderbook_json).unwrap();

    use crate::sources::binance::fetch_orderbook_depth;

    let orderbook = fetch_orderbook_depth("SOLUSDT", 1000).await.unwrap();

    let (grouped_bids, grouped_asks) = group_by_fractional_part(&orderbook, FractionalPart::One);

    let (top_asks, _) = top_n_support_resistance(&grouped_asks, 10);
    let (_, top_bids) = top_n_support_resistance(&grouped_bids, 10);

    let order_amount_asks_csv = to_csv(&top_asks);
    let order_amount_bids_csv = to_csv(&top_bids);

    println!("Asks CSV:\n{}", order_amount_asks_csv);
    println!("Bids CSV:\n{}", order_amount_bids_csv);

    // assert!(order_amount_bids_csv.contains("169.9,1287.643"));
    // assert!(order_amount_asks_csv.contains("170.0,542.225"));
}
