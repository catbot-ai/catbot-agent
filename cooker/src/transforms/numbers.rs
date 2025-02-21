use common::OrderBook;
use serde::Serialize;
use serde_json::{json, Value};
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

    println!("Grouped Bids: {:?}", grouped_bids);
    println!("Grouped Asks: {:?}", grouped_asks);

    (grouped_bids, grouped_asks)
}

type PriceAmountVec = Vec<(String, f64)>;

pub fn top_n_support_resistance(grouped_data: &BTreeMap<String, f64>, n: usize) -> PriceAmountVec {
    let mut price_amount_vec: Vec<PriceAmount> = grouped_data
        .iter()
        .filter_map(|(price_str, amount)| {
            if let Ok(price) = price_str.parse::<f64>() {
                Some(PriceAmount {
                    price,
                    cumulative_amount: *amount,
                })
            } else {
                eprintln!("Error parsing price: {}", price_str);
                None
            }
        })
        .collect();

    // Sort by cumulative_amount in descending order (highest volume first)
    price_amount_vec.sort_by(|a, b| {
        b.cumulative_amount
            .partial_cmp(&a.cumulative_amount)
            .unwrap()
    });

    let top_n_prices_amounts: PriceAmountVec = price_amount_vec
        .iter()
        .take(n)
        .map(|pa| (pa.price.to_string(), pa.cumulative_amount))
        .collect();

    top_n_prices_amounts
}

pub fn extract_prices_f64(price_amount_vec: &PriceAmountVec, n: usize) -> [f64; 3] {
    let mut prices_array = [0.0; 3]; // Initialize with default values

    for (i, price_amount) in price_amount_vec.iter().take(n).enumerate() {
        if let Ok(price) = price_amount.0.parse::<f64>() {
            // Access tuple element by index .0 (price string)
            prices_array[i] = price;
        } else {
            eprintln!("Error parsing price string: {}", price_amount.0);
        }
    }
    prices_array
}

pub fn btree_map_to_csv(grouped_data: &BTreeMap<String, f64>) -> String {
    let mut csv_string = String::new();
    csv_string.push_str("price,cumulative_amount\n"); // Add CSV header

    for (price_str, amount) in grouped_data.iter() {
        // Parse price_str to f64 for formatting (as in your to_csv function)
        if let Ok(price) = price_str.parse::<f64>() {
            csv_string.push_str(&format!("{:.0},{:.3}\n", price, amount));
        } else {
            eprintln!("Error parsing price: {}", price_str);
        }
    }
    csv_string
}

#[derive(Serialize)]
struct PriceAmount {
    price: f64,
    cumulative_amount: f64,
}

// #[cfg(test)]
// #[tokio::test]
// async fn test_group_and_top_n() {
//     // let orderbook_json = r#"{"lastUpdateId":18560646066,"bids":[["170.02000000","204.47900000"],["170.01000000","150.14900000"],["170.00000000","86.51000000"],["169.99000000","104.08900000"],["169.98000000","168.26600000"],["169.97000000","102.02100000"],["169.96000000","189.04000000"],["169.95000000","190.76100000"],["169.94000000","308.73800000"],["169.93000000","224.72800000"]],"asks":[["170.03000000","12.03800000"],["170.04000000","3.84100000"],["170.05000000","34.67200000"],["170.06000000","90.68600000"],["170.07000000","200.38200000"],["170.08000000","98.31900000"],["170.09000000","102.28700000"],["170.10000000","196.39600000"],["170.11000000","191.37100000"],["170.12000000","169.14700000"]]}"#;
//     // let orderbook: OrderBook = serde_json::from_str(orderbook_json).unwrap();

//     use crate::sources::binance::fetch_orderbook_depth;

//     let orderbook = fetch_orderbook_depth("SOLUSDT", 1000).await.unwrap();

//     let (grouped_bids, grouped_asks) = group_by_fractional_part(&orderbook, FractionalPart::One);

//     let (top_asks, _) = top_n_support_resistance(&grouped_asks, 10);
//     let (_, top_bids) = top_n_support_resistance(&grouped_bids, 10);

//     let order_amount_bids = to_json(&top_bids).to_string();
//     let order_amount_asks = to_json(&top_asks).to_string();

//     println!("Asks :\n{:#}", order_amount_bids);
//     println!("Bids :\n{:#}", order_amount_asks);

//     // assert!(order_amount_bids_csv.contains("169.9,1287.643"));
//     // assert!(order_amount_asks_csv.contains("170.0,542.225"));
// }
