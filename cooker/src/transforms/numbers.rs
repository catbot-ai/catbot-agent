use common::OrderBook;
use currency_rs::{Currency, CurrencyOpts};
use serde::Serialize;
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

type PriceAmountVec = Vec<(f64, f64)>;

pub fn top_n_bids_asks(
    grouped_data: &BTreeMap<String, f64>,
    n: usize,
    is_asks: bool,
) -> PriceAmountVec {
    let mut price_amount_vec: Vec<PriceAmount> = grouped_data
        .iter()
        .filter_map(|(price_str, amount)| {
            if let Ok(price) = price_str.parse::<f64>() {
                Some(PriceAmount {
                    price,
                    cumulative_amount: Currency::new_string(
                        &amount.to_string(),
                        Some(CurrencyOpts::new().set_symbol("").set_precision(3)),
                    )
                    .unwrap()
                    .to_string()
                    .parse::<f64>()
                    .unwrap(),
                })
            } else {
                eprintln!("Error parsing price: {}", price_str);
                None
            }
        })
        .collect();

    // Sort by price: ascending for asks, descending for bids
    price_amount_vec.sort_by(|a, b| {
        if is_asks {
            a.price.partial_cmp(&b.price).unwrap() // Ascending for asks
        } else {
            b.price.partial_cmp(&a.price).unwrap() // Descending for bids
        }
    });

    let top_n_prices_amounts: PriceAmountVec = price_amount_vec
        .iter()
        .take(n)
        .map(|pa| (pa.price, pa.cumulative_amount))
        .collect();

    top_n_prices_amounts
}

#[allow(unused)]
pub fn extract_prices_f64(price_amount_vec: &PriceAmountVec, n: usize) -> [f64; 3] {
    let mut prices_array = [0.0; 3];

    for (i, price_amount) in price_amount_vec.iter().take(n).enumerate() {
        prices_array[i] = price_amount.0;
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
