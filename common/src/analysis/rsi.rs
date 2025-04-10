use super::m4rs::kline_to_m4rs_candlestick;
use crate::Kline;
use anyhow::bail;
use m4rs::{bolinger_band, Candlestick};

pub fn calculate_stoch_rsi(
    candles: &[Candlestick],
    rsi_period: usize,
    stoch_period: usize,
    smooth_k: usize,
    smooth_d: usize,
) -> anyhow::Result<(Vec<u64>, Vec<f64>, Vec<f64>)> {
    // Step 1: Extract closing prices from M4rsCandlestick
    let closing_prices: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let closing_at: Vec<u64> = candles.iter().map(|c| c.at).collect();

    // Ensure there are enough candles for calculation
    if closing_prices.len() < rsi_period + stoch_period + smooth_k + smooth_d {
        bail!("Insufficient data for Stoch RSI calculation")
    }

    // Step 2: Calculate RSI (14 periods)
    let mut rsi = vec![0.0; closing_prices.len()];
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;

    // Initial RSI calculation (first 14 periods)
    for i in 1..rsi_period {
        let change = closing_prices[i] - closing_prices[i - 1];
        if change > 0.0 {
            avg_gain += change;
        } else {
            avg_loss += change.abs();
        }
    }
    avg_gain /= rsi_period as f64;
    avg_loss /= rsi_period as f64;

    // Wilder’s smoothing for RSI
    for i in rsi_period..closing_prices.len() {
        let change = closing_prices[i] - closing_prices[i - 1];
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { change.abs() } else { 0.0 };

        avg_gain = (avg_gain * (rsi_period - 1) as f64 + gain) / rsi_period as f64;
        avg_loss = (avg_loss * (rsi_period - 1) as f64 + loss) / rsi_period as f64;

        let rs = if avg_loss == 0.0 {
            100.0
        } else {
            avg_gain / avg_loss
        };
        rsi[i] = 100.0 - (100.0 / (1.0 + rs));
    }

    // Step 3: Calculate Stochastic RSI (14-period lookback)
    let mut stoch_rsi = vec![0.0; closing_prices.len()];
    for i in stoch_period..closing_prices.len() {
        let rsi_slice: Vec<f64> = rsi[(i - stoch_period + 1)..=i].to_vec();
        let lowest_rsi = rsi_slice.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let highest_rsi = rsi_slice.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        if highest_rsi == lowest_rsi {
            stoch_rsi[i] = if rsi[i] == lowest_rsi { 0.0 } else { 100.0 };
        } else {
            stoch_rsi[i] = (rsi[i] - lowest_rsi) / (highest_rsi - lowest_rsi) * 100.0;
        }
    }

    // Step 4: Smooth %K (3 periods)
    let mut smoothed_k = vec![0.0; closing_prices.len()];
    for i in smooth_k..closing_prices.len() {
        let k_slice: Vec<f64> = stoch_rsi[(i - smooth_k + 1)..=i].to_vec();
        smoothed_k[i] = k_slice.iter().sum::<f64>() / smooth_k as f64;
    }

    // Step 5: Calculate %D (3 periods)
    let mut d = vec![0.0; closing_prices.len()];
    for i in smooth_d..closing_prices.len() {
        let k_slice: Vec<f64> = smoothed_k[(i - smooth_d + 1)..=i].to_vec();
        d[i] = k_slice.iter().sum::<f64>() / smooth_d as f64;
    }

    Ok((closing_at, smoothed_k, d))
}

pub fn parse_stoch_rsi_csv(closing_at: &[u64], smoothed_k: &[f64], d: &[f64]) -> String {
    let mut csv_string = String::new();
    csv_string.push_str("at,stoch_rsi_k,stoch_rsi_d\n"); // Add CSV header

    // Ensure both vectors have the same length
    let len = smoothed_k.len().min(d.len());

    for i in 0..len {
        if smoothed_k[i] <= 0.0 || d[i] <= 0.0 {
            continue; // Skip this row if K or D is not positive
        }

        csv_string.push_str(&format!(
            "{},{:.2},{:.2}\n",
            closing_at[i], smoothed_k[i], d[i]
        ));
    }

    csv_string
}

pub fn get_stoch_rsi_csv(klines: &Vec<Kline>) -> anyhow::Result<String> {
    let m4rs_candlesticks = klines
        .iter()
        .map(kline_to_m4rs_candlestick)
        .collect::<Vec<_>>();
    let (closing_at, stoch_rsi_k, stoch_rsi_d) =
        calculate_stoch_rsi(&m4rs_candlesticks, 14, 14, 3, 3)?;
    let csv_string = parse_stoch_rsi_csv(&closing_at, &stoch_rsi_k, &stoch_rsi_d);
    Ok(csv_string)
}

pub fn parse_bb_csv(past_bb_lines: &Vec<(u64, f32, f32, f32)>) -> String {
    let mut csv_string = String::new();
    csv_string.push_str("at,avg,upper,lower\n"); // Add CSV header

    for &(at, avg, upper, lower) in past_bb_lines {
        csv_string.push_str(&format!("{},{:.2},{:.2},{:.2}\n", at, avg, upper, lower));
    }

    csv_string
}

pub fn get_bb_csv(klines: &Vec<Kline>) -> anyhow::Result<String> {
    let past_m4rs_candles: Vec<Candlestick> =
        klines.iter().map(kline_to_m4rs_candlestick).collect();
    let bb_result = bolinger_band(&past_m4rs_candles, 20)?;
    let bb_lines: Vec<(u64, f32, f32, f32)> = bb_result
        .iter()
        .map(|entry| {
            let t = entry.at;
            let avg = entry.avg as f32;
            let upper = (entry.avg + 2.0 * entry.sigma) as f32;
            let lower = (entry.avg - 2.0 * entry.sigma) as f32;
            (t, avg, upper, lower)
        })
        .collect();
    let csv_string = parse_bb_csv(&bb_lines);
    Ok(csv_string)
}

pub fn get_latest_bb_ma(klines: &[Kline]) -> anyhow::Result<String> {
    let past_m4rs_candles: Vec<Candlestick> =
        klines.iter().map(kline_to_m4rs_candlestick).collect();
    let bb_result = bolinger_band(&past_m4rs_candles, 20)?;
    let latest_bb = bb_result.last().unwrap();
    let ma_7 = past_m4rs_candles
        .iter()
        .rev()
        .take(7)
        .map(|c| c.close)
        .sum::<f64>()
        / 7.0;
    let ma_25 = past_m4rs_candles
        .iter()
        .rev()
        .take(25)
        .map(|c| c.close)
        .sum::<f64>()
        / 25.0;
    let ma_99 = past_m4rs_candles
        .iter()
        .rev()
        .take(99)
        .map(|c| c.close)
        .sum::<f64>()
        / 99.0;

    Ok(format!(
        "MA 7 close 0 SMA 9 {:.2}\nMA 25 close 0 SMA 9 {:.2}\nMA 99 close 0 SMA 9 {:.2}\nBB 20 2 {:.2} {:.2} {:.2}",
        ma_7, ma_25, ma_99, latest_bb.avg, latest_bb.avg + 2.0 * latest_bb.sigma, latest_bb.avg - 2.0 * latest_bb.sigma
    ))
}
