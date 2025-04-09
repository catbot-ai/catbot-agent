use std::collections::HashMap;

use anyhow::{Context, Result};
use futures::future::try_join_all;

use crate::{
    binance::{fetch_binance_kline_usdt, get_token_and_pair_symbol_usdt, klines_to_csv},
    rsi::get_many_stoch_rsi_csv,
    Kline,
};

// The Price History Builder
pub struct PriceHistoryBuilder<'a> {
    pair_symbol: &'a str,        // e.g. "SOL_USDT"
    binance_pair_symbol: String, // e.g. "SOLUSDT"
    limit: i32,
    kline_intervals: Vec<String>,
    stoch_rsi_intervals: Vec<String>,
    // Add fields for other indicators like MA, BB later
    // ma_intervals: Vec<String>,
    // bb_intervals: Vec<String>,
}

impl<'a> PriceHistoryBuilder<'a> {
    pub fn new(pair_symbol: &'a str, limit: i32) -> Self {
        let (_, binance_pair_symbol) = get_token_and_pair_symbol_usdt(pair_symbol);
        PriceHistoryBuilder {
            pair_symbol,
            binance_pair_symbol,
            limit,
            kline_intervals: Vec::new(),
            stoch_rsi_intervals: Vec::new(),
        }
    }

    // Helper to parse interval specifications using parse_interval_spec.
    // Returns a vector of (interval_name, optional_limit) tuples.
    // This can be used by methods like `with_klines` and `with_stoch_rsi`
    // if the internal storage is updated to Vec<(String, Option<i32>)>.
    #[allow(dead_code)] // This helper is not yet integrated into the builder logic.
    fn parse_interval_specs_list(&self, specs: &[&str]) -> Vec<(String, Option<i32>)> {
        specs.iter().map(|s| Self::parse_interval_spec(s)).collect()
    }

    // Helper function to parse interval specification strings like "1h" or "1h:200".
    // Returns the interval name (e.g., "1h") and an optional limit override.
    // This function itself doesn't implement the default logic (e.g., defaulting to 100);
    // the caller would handle the Option<i32> result, potentially using the
    // builder's main `limit` field as a default if this function returns None.
    //
    // Note: Integrating this requires changing how intervals are stored (e.g., Vec<(String, Option<i32>)>)
    // and how data is fetched in the `build` method to use the specific limit if provided.
    #[allow(dead_code)] // This helper is not yet integrated into the builder logic.
    fn parse_interval_spec(spec: &str) -> (String, Option<i32>) {
        if let Some((interval_part, limit_part)) = spec.rsplit_once(':') {
            // Check if the part after the colon is a valid positive integer
            if let Ok(limit) = limit_part.parse::<i32>() {
                if limit > 0 {
                    // Valid limit found
                    return (interval_part.to_string(), Some(limit));
                }
                // else: Invalid limit (e.g., "1h:0" or "1h:-5"), treat whole as interval
            }
            // else: Part after colon is not a number (e.g., "abc:xyz"), treat whole as interval
        }
        // No colon found, or part after colon was not a valid positive limit
        (spec.to_string(), None)
    }

    pub fn with_klines(mut self, intervals: &[&str]) -> Self {
        self.kline_intervals = intervals.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_stoch_rsi(mut self, intervals: &[&str]) -> Self {
        self.stoch_rsi_intervals = intervals.iter().map(|s| s.to_string()).collect();
        self
    }

    // Add methods like .with_ma(), .with_bb() here later

    // --- Format Klines ---
    fn format_klines_section(
        &self,
        kline_data_map: &HashMap<String, Vec<Kline>>,
    ) -> Result<String> {
        let mut klines_output = String::new();
        if !self.kline_intervals.is_empty() {
            klines_output.push_str("\n**Klines (Price History):**\n");
            // Sort intervals for consistent output order (optional but nice)
            let mut sorted_kline_intervals = self.kline_intervals.clone();
            // Basic sort is usually fine for typical intervals like "1m", "5m", "1h"
            sorted_kline_intervals.sort();

            for interval in &sorted_kline_intervals {
                if let Some(data) = kline_data_map.get(interval) {
                    if data.is_empty() {
                        klines_output.push_str(&format!(" ({}) No data found.\n", interval));
                        continue;
                    }
                    // Use the helper function klines_to_csv which returns Result<String>
                    match klines_to_csv(data) {
                        Ok(csv_data) => {
                            klines_output.push_str(&format!("\n* Interval: {}\n", interval));
                            klines_output.push_str("```csv\n");
                            klines_output.push_str(&csv_data);
                            klines_output.push_str("```\n");
                        }
                        Err(e) => {
                            klines_output.push_str(&format!(
                                "\n* Interval: {} (Error formatting Klines to CSV: {})\n",
                                interval, e
                            ));
                            eprintln!("Error formatting klines to CSV for {}: {}", interval, e);
                        }
                    }
                } else {
                    // This case should ideally not be reached if try_join_all succeeded
                    klines_output.push_str(&format!(
                        "\n* Interval: {} (Data unexpectedly missing after fetch)\n",
                        interval
                    ));
                    eprintln!(
                        "Warning: Kline data for interval {} requested but not found in map.",
                        interval
                    );
                }
            }
        }
        Ok(klines_output)
    }

    // --- Format StochRSI ---
    fn format_stoch_rsi_section(
        &self,
        kline_data_map: &HashMap<String, Vec<Kline>>,
    ) -> Result<String> {
        let mut stoch_rsi_output = String::new();
        if !self.stoch_rsi_intervals.is_empty() {
            stoch_rsi_output.push_str("\n**Stochastic RSI:**\n");
            // Sort intervals for consistent output order (optional but nice)
            let mut sorted_stoch_rsi_intervals = self.stoch_rsi_intervals.clone();
            sorted_stoch_rsi_intervals.sort();

            for interval in &sorted_stoch_rsi_intervals {
                if let Some(data) = kline_data_map.get(interval) {
                    if data.is_empty() {
                        stoch_rsi_output.push_str(&format!(
                            " ({}) No kline data available to calculate StochRSI.\n",
                            interval
                        ));
                        continue;
                    }

                    // Call the function based on the diagnostic information: takes &Vec<Kline>, returns Result<String>
                    match get_many_stoch_rsi_csv(data) {
                        Ok(stoch_rsi_csv) => {
                            // Handles the Result::Ok case
                            stoch_rsi_output.push_str(&format!("\n* Interval: {}\n", interval));
                            stoch_rsi_output.push_str("```csv\n");
                            stoch_rsi_output.push_str(&stoch_rsi_csv); // Use the String from Ok(...)
                            stoch_rsi_output.push_str("```\n");
                        }
                        Err(e) => {
                            // Handles the Result::Err case
                            stoch_rsi_output.push_str(&format!(
                                "\n* Interval: {} (Error calculating StochRSI: {})\n",
                                interval, e
                            ));
                            eprintln!("Error calculating StochRSI for {}: {}", interval, e);
                        }
                    }
                } else {
                    // This case should ideally not be reached
                    stoch_rsi_output.push_str(&format!("\n* Interval: {} (Kline data unexpectedly missing for StochRSI calculation)\n", interval));
                    eprintln!("Warning: Kline data for interval {} needed for StochRSI but not found in map.", interval);
                }
            }
        }
        Ok(stoch_rsi_output)
    }

    /// Fetches all required data concurrently and formats it into a single string.
    pub async fn build(&self) -> Result<String> {
        let mut output_string = String::new();
        output_string.push_str("## Historical Data:\n");

        // Determine unique intervals needed for fetching Kline data
        let mut all_intervals = self.kline_intervals.clone();
        all_intervals.extend(self.stoch_rsi_intervals.clone());
        // Add intervals for MA, BB etc. here if needed
        all_intervals.sort();
        all_intervals.dedup();

        if all_intervals.is_empty() {
            output_string.push_str("No historical data requested.\n");
            return Ok(output_string);
        }

        // --- Fetch all necessary Kline data concurrently ---
        // Check if running in a WASM environment that supports threads/spawning.
        // Cloudflare Workers std::thread::spawn is not supported.
        // `try_join_all` itself doesn't spawn threads but drives futures concurrently
        // on the single thread's executor in wasm-bindgen contexts, which is generally fine.
        let fetch_futures = all_intervals.iter().map(|interval| {
            // Clone necessary data for the async block
            let binance_pair_symbol = self.binance_pair_symbol.clone();
            let interval = interval.clone();
            let limit = self.limit;
            // Capture pair_symbol for potential use in fetch_binance_kline_usdt if needed
            let pair_symbol = self.pair_symbol.to_string();

            async move {
                // Call the original function which constructs the URL and handles the request
                // We pass the *original* pair_symbol (e.g., "SOL_USDT") as it might be used internally,
                // even though binance_pair_symbol (e.g., "SOLUSDT") is derived and likely used for the API call.
                // Ensure fetch_binance_kline_usdt is compatible with this call pattern.
                // The diagnostics didn't complain about this part, so assuming it's correct.
                let kline_data: Vec<Kline> =
                    // Assuming fetch_binance_kline_usdt uses the pair_symbol to derive the binance_pair_symbol internally.
                    // If it DIRECTLY needs the binance_pair_symbol, pass that instead. Let's stick to the original code's apparent usage.
                    fetch_binance_kline_usdt::<Kline>(&pair_symbol, &interval, limit)
                        .await
                        .with_context(|| {
                            format!(
                                "Failed fetching klines for {} ({}) interval {}",
                                pair_symbol, binance_pair_symbol, interval
                            )
                        })?;

                Ok::<_, anyhow::Error>((interval, kline_data))
            }
        });

        // Execute all fetches concurrently. This works on single-threaded WASM runtimes.
        let fetched_kline_results: Vec<(String, Vec<Kline>)> = try_join_all(fetch_futures).await?;

        // Convert the Vec of tuples into a HashMap
        let kline_data_map: HashMap<String, Vec<Kline>> =
            fetched_kline_results.into_iter().collect();

        println!(
            "Builder fetched kline data for intervals: {:?}",
            kline_data_map.keys()
        );

        // --- Format Klines ---
        output_string.push_str(&self.format_klines_section(&kline_data_map)?);

        // --- Format StochRSI ---
        output_string.push_str(&self.format_stoch_rsi_section(&kline_data_map)?);

        Ok(output_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tokio;

    #[tokio::test]
    async fn test_price_history_builder_build() -> Result<()> {
        // Define test parameters
        let pair_symbol = "SOL_USDT";
        let limit = 50; // Fetch a reasonable number for testing
        let kline_intervals = ["1h", "4h"];
        let stoch_rsi_intervals = ["1h"];

        // Create and configure the builder
        let builder = PriceHistoryBuilder::new(pair_symbol, limit)
            .with_klines(&kline_intervals)
            .with_stoch_rsi(&stoch_rsi_intervals);

        // Build the historical data string
        let result_string = builder.build().await?;

        println!("--- Price History Builder Output ---");
        println!("{}", result_string);
        println!("--- End Price History Builder Output ---");

        // Assertions
        assert!(result_string.starts_with("## Historical Data:\n"));

        // Check for Kline sections
        assert!(result_string.contains("\n**Klines (Price History):**\n"));
        assert!(result_string.contains(
            "\n* Interval: 1h\n```csv\nopen_time,open,high,low,close,volume,close_time\n"
        ));
        assert!(result_string.contains(
            "\n* Interval: 4h\n```csv\nopen_time,open,high,low,close,volume,close_time\n"
        ));
        // Check if some data rows are present (simple check for non-empty data)
        assert!(result_string.matches("\n").count() > 10); // Expecting headers + data rows

        // Check for StochRSI section
        assert!(result_string.contains("\n**Stochastic RSI:**\n"));

        // Find the start of the StochRSI CSV block for the 1h interval
        let stoch_rsi_header = "\n* Interval: 1h\n```csv\nat,stoch_rsi_k,stoch_rsi_d\n";
        assert!(result_string.contains(stoch_rsi_header));

        let stoch_rsi_block_start = result_string.find(stoch_rsi_header);
        assert!(
            stoch_rsi_block_start.is_some(),
            "StochRSI 1h header not found"
        );

        if let Some(start_index) = stoch_rsi_block_start {
            let block_content_start = start_index + stoch_rsi_header.len();
            // Find the end of the CSV block
            let block_content_end = result_string[block_content_start..].find("```\n");
            assert!(
                block_content_end.is_some(),
                "StochRSI 1h CSV block end not found"
            );

            if let Some(end_offset) = block_content_end {
                let stoch_rsi_data =
                    &result_string[block_content_start..block_content_start + end_offset];
                // Check if there's at least one newline in the data part, indicating at least one data row
                // The calculation might not produce data for the full 'limit' length due to lookback periods.
                // Check if the data string is not empty or just whitespace.
                assert!(
                    stoch_rsi_data.trim().lines().count() > 0,
                    "StochRSI 1h data rows appear to be missing or empty. Data fetched: '{}'",
                    stoch_rsi_data
                );
                println!(
                    "StochRSI Data Block (first 100 chars): {:.100}",
                    stoch_rsi_data.trim()
                ); // Log snippet
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_price_history_builder_empty() -> Result<()> {
        // Define test parameters
        let pair_symbol = "SOL_USDT";
        let limit = 10;

        // Create builder without specifying intervals
        let builder = PriceHistoryBuilder::new(pair_symbol, limit);

        // Build the historical data string
        let result_string = builder.build().await?;

        println!("--- Empty Builder Output ---");
        println!("{}", result_string);
        println!("--- End Empty Builder Output ---");

        // Assertions
        assert_eq!(
            result_string,
            "## Historical Data:\nNo historical data requested.\n"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_price_history_builder_klines_only() -> Result<()> {
        // Define test parameters
        let pair_symbol = "SOL_USDT";
        let limit = 20;
        let kline_intervals = ["5m"];

        // Create and configure the builder
        let builder = PriceHistoryBuilder::new(pair_symbol, limit).with_klines(&kline_intervals);

        // Build the historical data string
        let result_string = builder.build().await?;

        println!("--- Klines Only Builder Output ---");
        println!("{}", result_string);
        println!("--- End Klines Only Builder Output ---");

        // Assertions
        assert!(result_string.starts_with("## Historical Data:\n"));
        assert!(result_string.contains("\n**Klines (Price History):**\n"));
        assert!(result_string.contains(
            "\n* Interval: 5m\n```csv\nopen_time,open,high,low,close,volume,close_time\n"
        ));
        assert!(!result_string.contains("\n**Stochastic RSI:**\n")); // Should not contain StochRSI

        Ok(())
    }
}
