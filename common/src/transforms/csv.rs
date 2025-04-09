use std::collections::HashMap;

use anyhow::{Context, Result};
use futures::future::try_join_all;

use crate::{
    binance::{fetch_binance_kline_usdt, klines_to_csv},
    rsi::get_many_stoch_rsi_csv,
    Kline,
};

// Helper function to parse interval specification strings like "1h" or "1h:200".
// Returns the interval name (e.g., "1h") and an optional limit override.
fn parse_interval_spec(spec: &str) -> (String, Option<i32>) {
    if let Some((interval_part, limit_part)) = spec.rsplit_once(':') {
        // Check if the part after the colon is a valid positive integer
        if let Ok(limit) = limit_part.parse::<i32>() {
            if limit > 0 {
                // Valid limit found
                return (interval_part.to_string(), Some(limit));
            }
            // else: Invalid limit (e.g., "1h:0" or "1h:-5"), treat whole as interval
            println!(
                "Warning: Invalid limit '{}' in spec '{}'. Treating whole as interval name.",
                limit_part, spec
            );
        }
        // else: Part after colon is not a number (e.g., "abc:xyz"), treat whole as interval
    }
    // No colon found, or part after colon was not a valid positive limit
    (spec.to_string(), None)
}

// Helper to parse a list of interval specifications using parse_interval_spec.
// Returns a vector of (interval_name, optional_limit) tuples.
fn parse_interval_specs_list(specs: &[&str]) -> Vec<(String, Option<i32>)> {
    specs.iter().map(|s| parse_interval_spec(s)).collect()
}

// The Price History Builder
pub struct PriceHistoryBuilder<'a> {
    pair_symbol: &'a str, // e.g. "SOL_USDT"
    default_limit: i32,   // Default limit if not specified per interval
    // Store intervals as (name, optional_limit)
    kline_intervals: Vec<(String, Option<i32>)>,
    stoch_rsi_intervals: Vec<(String, Option<i32>)>,
    // Add fields for other indicators later
    // ma_intervals: Vec<(String, Option<i32>)>,
    // bb_intervals: Vec<(String, Option<i32>)>,
}

impl<'a> PriceHistoryBuilder<'a> {
    /// Creates a new PriceHistoryBuilder.
    ///
    /// # Arguments
    ///
    /// * `pair_symbol` - The trading pair symbol (e.g., "SOL_USDT").
    /// * `default_limit` - The default number of klines to fetch if not specified per interval.
    pub fn new(pair_symbol: &'a str, default_limit: i32) -> Self {
        PriceHistoryBuilder {
            pair_symbol,
            default_limit,
            kline_intervals: Vec::new(),
            stoch_rsi_intervals: Vec::new(),
        }
    }

    /// Adds Kline intervals to fetch.
    /// Intervals can be specified like "1h" or "1h:200" to override the default limit.
    pub fn with_klines(mut self, intervals: &[&str]) -> Self {
        self.kline_intervals = parse_interval_specs_list(intervals);
        self
    }

    /// Adds Stochastic RSI intervals to calculate.
    /// The underlying Kline data will be fetched based on the specified interval.
    /// Intervals can be specified like "1h" or "1h:150". The limit affects the
    /// underlying Kline data fetch.
    pub fn with_stoch_rsi(mut self, intervals: &[&str]) -> Self {
        self.stoch_rsi_intervals = parse_interval_specs_list(intervals);
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
            // Sort intervals by name for consistent output order
            let mut sorted_kline_intervals = self.kline_intervals.clone();
            sorted_kline_intervals.sort_by(|a, b| a.0.cmp(&b.0)); // Compare interval names (String)

            for (interval_name, opt_limit) in &sorted_kline_intervals {
                let display_interval = match opt_limit {
                    Some(limit) => format!("{}:{}", interval_name, limit),
                    None => interval_name.clone(),
                };

                if let Some(data) = kline_data_map.get(interval_name) {
                    if data.is_empty() {
                        klines_output
                            .push_str(&format!(" ({}) No data found.\n", display_interval));
                        continue;
                    }
                    match klines_to_csv(data) {
                        Ok(csv_data) => {
                            klines_output
                                .push_str(&format!("\n* Interval: {}\n", display_interval));
                            klines_output.push_str("```csv\n");
                            klines_output.push_str(&csv_data);
                            klines_output.push_str("```\n");
                        }
                        Err(e) => {
                            klines_output.push_str(&format!(
                                "\n* Interval: {} (Error formatting Klines to CSV: {})\n",
                                display_interval, e
                            ));
                            eprintln!(
                                "Error formatting klines to CSV for {}: {}",
                                interval_name,
                                e // Log with base interval name
                            );
                        }
                    }
                } else {
                    klines_output.push_str(&format!(
                        "\n* Interval: {} (Data unexpectedly missing after fetch)\n",
                        display_interval
                    ));
                    eprintln!(
                        "Warning: Kline data for interval {} requested but not found in map.",
                        interval_name
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
            // Sort intervals by name for consistent output order
            let mut sorted_stoch_rsi_intervals = self.stoch_rsi_intervals.clone();
            sorted_stoch_rsi_intervals.sort_by(|a, b| a.0.cmp(&b.0)); // Compare interval names

            for (interval_name, opt_limit) in &sorted_stoch_rsi_intervals {
                let display_interval = match opt_limit {
                    Some(limit) => format!("{}:{}", interval_name, limit),
                    None => interval_name.clone(),
                };

                if let Some(data) = kline_data_map.get(interval_name) {
                    if data.is_empty() {
                        stoch_rsi_output.push_str(&format!(
                            " ({}) No kline data available to calculate StochRSI.\n",
                            display_interval
                        ));
                        continue;
                    }

                    match get_many_stoch_rsi_csv(data) {
                        Ok(stoch_rsi_csv) => {
                            stoch_rsi_output
                                .push_str(&format!("\n* Interval: {}\n", display_interval));
                            stoch_rsi_output.push_str("```csv\n");
                            stoch_rsi_output.push_str(&stoch_rsi_csv);
                            stoch_rsi_output.push_str("```\n");
                        }
                        Err(e) => {
                            stoch_rsi_output.push_str(&format!(
                                "\n* Interval: {} (Error calculating StochRSI: {})\n",
                                display_interval, e
                            ));
                            eprintln!(
                                "Error calculating StochRSI for {}: {}",
                                interval_name,
                                e // Log with base interval name
                            );
                        }
                    }
                } else {
                    stoch_rsi_output.push_str(&format!("\n* Interval: {} (Kline data unexpectedly missing for StochRSI calculation)\n", display_interval));
                    eprintln!("Warning: Kline data for interval {} needed for StochRSI but not found in map.", interval_name);
                }
            }
        }
        Ok(stoch_rsi_output)
    }

    /// Fetches all required data concurrently and formats it into a single string.
    pub async fn build(&self) -> Result<String> {
        let mut output_string = String::new();
        output_string.push_str("## Historical Data:\n");

        // --- Determine unique intervals and their effective limits for fetching ---
        let mut all_interval_specs = self.kline_intervals.clone();
        all_interval_specs.extend(self.stoch_rsi_intervals.clone());
        // Add intervals for MA, BB etc. here if needed:
        // all_interval_specs.extend(self.ma_intervals.clone());

        // Use a HashMap to find the highest limit requested for each unique interval name.
        // This ensures we fetch enough data if an interval is requested multiple times
        // (e.g., once for klines with default limit, once for RSI with a specific limit).
        let mut effective_fetch_params: HashMap<String, i32> = HashMap::new(); // Explicit type annotation might help, though entry API should infer.
        for (name, opt_limit) in &all_interval_specs {
            let required_limit = opt_limit.unwrap_or(self.default_limit);
            effective_fetch_params
                .entry(name.clone())
                // Add explicit type annotation for the closure parameter
                .and_modify(|current_limit: &mut i32| {
                    *current_limit = (*current_limit).max(required_limit)
                })
                .or_insert(required_limit);
        }

        if effective_fetch_params.is_empty() {
            output_string.push_str("No historical data requested.\n");
            return Ok(output_string);
        }

        println!("Effective fetch params: {:?}", effective_fetch_params); // Debugging

        // --- Fetch all necessary Kline data concurrently ---
        let fetch_futures = effective_fetch_params
            .iter()
            .map(|(interval_name, &limit_to_use)| {
                // Clone necessary data for the async block
                let interval = interval_name.clone();
                let pair_symbol = self.pair_symbol.to_string(); // Original symbol

                async move {
                    let kline_data: Vec<Kline> =
                        fetch_binance_kline_usdt::<Kline>(&pair_symbol, &interval, limit_to_use)
                            .await
                            .with_context(|| {
                                format!(
                                    "Failed fetching klines for {} interval {} with limit {}",
                                    pair_symbol,
                                    interval,
                                    limit_to_use // Include limit in error
                                )
                            })?;

                    Ok::<_, anyhow::Error>((interval, kline_data)) // Return interval name and data
                }
            });

        // Execute all fetches concurrently
        let fetched_kline_results: Vec<(String, Vec<Kline>)> = try_join_all(fetch_futures).await?;

        // Convert the Vec of tuples into a HashMap for easy lookup
        let kline_data_map: HashMap<String, Vec<Kline>> =
            fetched_kline_results.into_iter().collect();

        println!(
            "Builder fetched kline data for intervals: {:?}",
            kline_data_map.keys()
        );

        // --- Format Klines ---
        // Pass the fetched data map to the formatting function
        output_string.push_str(&self.format_klines_section(&kline_data_map)?);

        // --- Format StochRSI ---
        // Pass the same fetched data map
        output_string.push_str(&self.format_stoch_rsi_section(&kline_data_map)?);

        // --- Format other indicators here ---

        Ok(output_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tokio;

    // Helper to check CSV structure (header + at least one data row)
    fn check_csv_block(content: &str, header: &str) -> bool {
        if let Some(block_start) = content.find(header) {
            let data_start = block_start + header.len();
            if let Some(block_end) = content[data_start..].find("```\n") {
                let csv_data = &content[data_start..data_start + block_end];
                return csv_data.trim().contains('\n'); // Check for at least one newline after header
            }
        }
        false
    }

    #[tokio::test]
    async fn test_price_history_builder_basic() -> Result<()> {
        let pair_symbol = "SOL_USDT"; // Use a common pair
        let default_limit = 50;
        let kline_intervals = ["1h", "4h"];
        let stoch_rsi_intervals = ["1h"]; // Uses fetched 1h klines

        let builder = PriceHistoryBuilder::new(pair_symbol, default_limit)
            .with_klines(&kline_intervals)
            .with_stoch_rsi(&stoch_rsi_intervals);

        let result_string = builder.build().await?;
        println!("--- Basic Test Output ---\n{}\n--- End ---", result_string);

        assert!(result_string.starts_with("## Historical Data:\n"));
        assert!(result_string.contains("\n**Klines (Price History):**\n"));
        assert!(result_string.contains("\n* Interval: 1h\n")); // Display name without limit
        assert!(check_csv_block(
            &result_string,
            "```csv\nopen_time,open,high,low,close,volume,close_time\n"
        ));
        assert!(result_string.contains("\n* Interval: 4h\n"));
        assert!(check_csv_block(
            &result_string,
            "```csv\nopen_time,open,high,low,close,volume,close_time\n"
        ));

        assert!(result_string.contains("\n**Stochastic RSI:**\n"));
        assert!(result_string.contains("\n* Interval: 1h\n")); // Display name without limit
        assert!(check_csv_block(
            &result_string,
            "```csv\nat,stoch_rsi_k,stoch_rsi_d\n"
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_price_history_builder_with_limit_override() -> Result<()> {
        let pair_symbol = "SOL_USDT";
        let default_limit = 100; // Default
                                 // Request 1h klines with specific limit, 4h with default, and 1h StochRSI (implies 1h klines) with specific limit
        let kline_intervals = ["1h:30", "4h"]; // Override 1h limit, use default for 4h
        let stoch_rsi_intervals = ["1h:50"]; // Request 1h StochRSI, needing 50 klines

        // Builder should detect that 1h klines are needed with limit 30 and limit 50,
        // so it should fetch 1h klines with the *maximum* required limit (50).
        let builder = PriceHistoryBuilder::new(pair_symbol, default_limit)
            .with_klines(&kline_intervals)
            .with_stoch_rsi(&stoch_rsi_intervals);

        let result_string = builder.build().await?;
        println!(
            "--- Limit Override Test Output ---\n{}\n--- End ---",
            result_string
        );

        assert!(result_string.starts_with("## Historical Data:\n"));
        assert!(result_string.contains("\n**Klines (Price History):**\n"));
        // Check that the display reflects the requested specification
        assert!(result_string.contains("\n* Interval: 1h:30\n"));
        assert!(check_csv_block(
            &result_string,
            "```csv\nopen_time,open,high,low,close,volume,close_time\n"
        ));
        assert!(result_string.contains("\n* Interval: 4h\n")); // Uses default limit, display doesn't show it
        assert!(check_csv_block(
            &result_string,
            "```csv\nopen_time,open,high,low,close,volume,close_time\n"
        ));

        assert!(result_string.contains("\n**Stochastic RSI:**\n"));
        // Check that the display reflects the requested specification
        assert!(result_string.contains("\n* Interval: 1h:50\n"));
        assert!(check_csv_block(
            &result_string,
            "```csv\nat,stoch_rsi_k,stoch_rsi_d\n"
        ));

        // We can't easily verify the *exact* number of rows fetched without mocking,
        // but the structure and headers reflecting the *specified* intervals (with limits) should be present.
        // The underlying fetch for "1h" should have used limit=50.

        Ok(())
    }

    #[tokio::test]
    async fn test_price_history_builder_empty() -> Result<()> {
        let pair_symbol = "SOL_USDT";
        let default_limit = 10;
        let builder = PriceHistoryBuilder::new(pair_symbol, default_limit); // No intervals added
        let result_string = builder.build().await?;

        println!(
            "--- Empty Builder Output ---\n{}\n--- End ---",
            result_string
        );
        assert_eq!(
            result_string,
            "## Historical Data:\nNo historical data requested.\n"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_price_history_builder_klines_only_mixed_limits() -> Result<()> {
        let pair_symbol = "SOL_USDT";
        let default_limit = 50;
        let kline_intervals = ["5m", "15m:30", "1h:70"]; // Mixed specifications

        let builder =
            PriceHistoryBuilder::new(pair_symbol, default_limit).with_klines(&kline_intervals);
        let result_string = builder.build().await?;
        println!(
            "--- Klines Only Mixed Limits Output ---\n{}\n--- End ---",
            result_string
        );

        assert!(result_string.starts_with("## Historical Data:\n"));
        assert!(result_string.contains("\n**Klines (Price History):**\n"));
        assert!(result_string.contains("\n* Interval: 5m\n"));
        assert!(result_string.contains("\n* Interval: 15m:30\n"));
        assert!(result_string.contains("\n* Interval: 1h:70\n"));
        assert!(!result_string.contains("\n**Stochastic RSI:**\n")); // No RSI requested

        // Helper function to extract CSV data for a specific interval display name
        let extract_csv_data = |content: &str, interval_display: &str| -> Option<String> {
            let block_start_marker = format!("\n* Interval: {}\n```csv\n", interval_display);
            let header = "open_time,open,high,low,close,volume,close_time\n";
            if let Some(block_start) = content.find(&block_start_marker) {
                let csv_start = block_start + block_start_marker.len();
                // Ensure the header is present right after the marker
                if content[csv_start..].starts_with(header) {
                    let data_start = csv_start + header.len();
                    if let Some(block_end) = content[data_start..].find("\n```\n") {
                        return Some(content[data_start..data_start + block_end].to_string());
                    }
                }
            }
            None
        };

        // Check row count for 15m:30
        if let Some(csv_data_15m) = extract_csv_data(&result_string, "15m:30") {
            let row_count = csv_data_15m.trim().lines().count();
            // Cast the expected i32 limit to usize for comparison
            assert_eq!(
                row_count, 30usize,
                "Expected 30 rows for interval 15m:30, found {}",
                row_count
            );
            println!("Found {} rows for 15m:30 (expected 30)", row_count); // Debug print
        } else {
            panic!("CSV data block for interval '15m:30' not found or malformed");
        }

        // Optionally, check other intervals
        if let Some(csv_data_5m) = extract_csv_data(&result_string, "5m") {
            let row_count = csv_data_5m.trim().lines().count();
            // Cast the expected i32 limit to usize for comparison
            let expected_rows: usize = default_limit
                .try_into()
                .expect("Default limit should be convertible to usize");
            assert_eq!(
                row_count, expected_rows,
                "Expected {} (default limit) rows for interval 5m, found {}",
                default_limit, row_count
            );
            println!(
                "Found {} rows for 5m (expected {})",
                row_count, default_limit
            ); // Debug print
        } else {
            panic!("CSV data block for interval '5m' not found or malformed");
        }

        if let Some(csv_data_1h) = extract_csv_data(&result_string, "1h:70") {
            let row_count = csv_data_1h.trim().lines().count();
            // Cast the expected i32 limit to usize for comparison
            assert_eq!(
                row_count, 70usize,
                "Expected 70 rows for interval 1h:70, found {}",
                row_count
            );
            println!("Found {} rows for 1h:70 (expected 70)", row_count); // Debug print
        } else {
            panic!("CSV data block for interval '1h:70' not found or malformed");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_limit_spec_falls_back_to_interval_name() -> Result<()> {
        let pair_symbol = "SOL_USDT";
        let default_limit = 20;
        // "1h:abc" is invalid, should be treated as interval "1h:abc"
        // "30m:0" is invalid (limit <= 0), should be treated as interval "30m:0"
        let kline_intervals = ["1h:abc", "30m:0", "4h"]; // 4h uses default limit

        // Expect warnings to be printed during parsing (can't easily capture stderr in test)
        let builder =
            PriceHistoryBuilder::new(pair_symbol, default_limit).with_klines(&kline_intervals);
        let result_string = builder.build().await?;
        println!(
            "--- Invalid Spec Fallback Test Output ---\n{}\n--- End ---",
            result_string
        );

        assert!(result_string.contains("\n**Klines (Price History):**\n"));
        // Check if the invalid specs are treated as literal interval names
        // Note: Binance likely won't have data for "1h:abc" or "30m:0", so expect errors or "No data"
        assert!(result_string.contains("\n* Interval: 1h:abc")); // Check interval display name
        assert!(result_string.contains("\n* Interval: 30m:0")); // Check interval display name
        assert!(result_string.contains("\n* Interval: 4h\n")); // Valid interval should work

        // Expect "No data found" or similar for the invalid interval names, as the fetch will likely fail.
        // We'll check that the valid one (4h) has data.
        let four_h_section_start = result_string.find("\n* Interval: 4h\n").unwrap();
        let four_h_section = &result_string[four_h_section_start..];
        assert!(check_csv_block(
            four_h_section,
            "```csv\nopen_time,open,high,low,close,volume,close_time\n"
        ));

        Ok(())
    }
}
