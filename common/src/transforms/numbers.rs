use crate::OrderBook;
use std::collections::{BTreeMap, HashMap};
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

pub fn group_by_fractional_part_f32(
    orderbook_data: &OrderBook,
    fractional_part: FractionalPart,
) -> (HashMap<u32, f64>, HashMap<u32, f64>) {
    let mut grouped_bids: HashMap<u32, f64> = HashMap::new();
    let mut grouped_asks: HashMap<u32, f64> = HashMap::new();

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
                if price.is_finite() {
                    grouped_bids.insert((price as f32).to_bits(), amount_str);
                }
            }
        }
    }

    for ask in &orderbook_data.asks {
        if ask.len() == 2 {
            if let (Ok(price_str), Ok(amount_str)) = (ask[0].parse::<f64>(), ask[1].parse::<f64>())
            {
                let price = (price_str * multiplier).ceil() / multiplier;
                if price.is_finite() {
                    grouped_asks.insert((price as f32).to_bits(), amount_str);
                }
            }
        }
    }

    (grouped_bids, grouped_asks)
}

pub fn convert_grouped_data(
    grouped_bids: &HashMap<u32, f64>,
    grouped_asks: &HashMap<u32, f64>,
    min_price: f32,
    max_price: f32,
) -> (HashMap<u32, f32>, HashMap<u32, f32>) {
    let mut bid_volumes: HashMap<u32, f32> = HashMap::new();
    let mut ask_volumes: HashMap<u32, f32> = HashMap::new();

    for (price_bits, volume) in grouped_bids.iter() {
        let price = f32::from_bits(*price_bits);
        if price >= min_price && price <= max_price && price.is_finite() {
            bid_volumes.insert(*price_bits, *volume as f32);
        }
    }

    for (price_bits, volume) in grouped_asks.iter() {
        let price = f32::from_bits(*price_bits);
        if price >= min_price && price <= max_price && price.is_finite() {
            ask_volumes.insert(*price_bits, *volume as f32);
        }
    }

    (bid_volumes, ask_volumes)
}

struct PriceAmount {
    price: f64,
    cumulative_amount: f64,
}

pub fn top_n_bids_asks(
    grouped_data: &BTreeMap<String, f64>,
    n: usize,
    is_asks: bool,
) -> Vec<Vec<f64>> {
    let mut price_amount_vec: Vec<PriceAmount> = grouped_data
        .iter()
        .filter_map(|(price_str, amount)| {
            if let Ok(price) = price_str.parse::<f64>() {
                if let Ok(amount_f64) = amount.to_string().parse::<f64>() {
                    Some(PriceAmount {
                        price,
                        cumulative_amount: amount_f64,
                    })
                } else {
                    eprintln!("Error parsing amount: {}", amount);
                    None
                }
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

    let top_n_prices_amounts: Vec<Vec<f64>> = price_amount_vec
        .iter()
        .take(n)
        .map(|pa| vec![pa.price, pa.cumulative_amount])
        .collect();

    top_n_prices_amounts
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
