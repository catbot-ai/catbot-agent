use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::{
    binance::{fetch_binance_kline_usdt, klines_to_csv},
    rsi::{get_latest_bb_ma, get_stoch_rsi_csv},
    Kline,
};

// Helper function to parse interval specification strings like "1h" or "1h:200".
// Returns the interval name (e.g., "1h") and an optional limit override.
fn parse_interval_spec(spec: &str) -> (String, Option<i32>) {
    if let Some((interval_part, limit_part)) = spec.rsplit_once(':') {
        if let Ok(limit) = limit_part.parse::<i32>() {
            if limit > 0 {
                return (interval_part.to_string(), Some(limit));
            }
            println!(
                "Warning: Invalid limit '{}' in spec '{}'. Treating whole as interval name.",
                limit_part, spec
            );
        }
    }
    (spec.to_string(), None)
}

// Helper to parse a list of interval specifications using parse_interval_spec.
// Returns a vector of (interval_name, optional_limit) tuples.
fn parse_interval_specs_list(specs: &[&str]) -> Vec<(String, Option<i32>)> {
    specs.iter().map(|s| parse_interval_spec(s)).collect()
}

// The Price History Builder
pub struct PriceHistoryBuilder<'a> {
    pair_symbol: &'a str,
    default_limit: i32,
    kline_intervals: Vec<(String, Option<i32>)>,
    stoch_rsi_intervals: Vec<(String, Option<i32>)>,
    bb_intervals: Vec<(String, Option<i32>)>,
    ma_intervals: Vec<(String, Option<i32>)>,
    latest_bb_ma_intervals: Vec<(String, Option<i32>)>,
}

impl<'a> PriceHistoryBuilder<'a> {
    /// Creates a new PriceHistoryBuilder.
    pub fn new(pair_symbol: &'a str, default_limit: i32) -> Self {
        PriceHistoryBuilder {
            pair_symbol,
            default_limit,
            kline_intervals: Vec::new(),
            stoch_rsi_intervals: Vec::new(),
            bb_intervals: Vec::new(),
            ma_intervals: Vec::new(),
            latest_bb_ma_intervals: Vec::new(),
        }
    }

    /// Adds Kline intervals to fetch. Can be called multiple times.
    pub fn with_klines(mut self, intervals: &[&str]) -> Self {
        self.kline_intervals
            .extend(parse_interval_specs_list(intervals));
        self
    }

    /// Adds Stochastic RSI intervals to calculate. Can be called multiple times.
    pub fn with_stoch_rsi(mut self, intervals: &[&str]) -> Self {
        self.stoch_rsi_intervals
            .extend(parse_interval_specs_list(intervals));
        self
    }

    pub fn with_bb(mut self, intervals: &[&str]) -> Self {
        self.bb_intervals
            .extend(parse_interval_specs_list(intervals));
        self
    }

    pub fn with_latest_bb_ma(mut self, intervals: &[&str]) -> Self {
        self.latest_bb_ma_intervals
            .extend(parse_interval_specs_list(intervals));
        self
    }

    /// Fetches the required Kline data sequentially, one interval at a time.
    async fn fetch_each_intervals(&self) -> Result<HashMap<String, Vec<Kline>>> {
        let mut all_interval_specs = self.kline_intervals.clone();
        all_interval_specs.extend(self.stoch_rsi_intervals.clone());
        all_interval_specs.extend(self.bb_intervals.clone());
        all_interval_specs.extend(self.latest_bb_ma_intervals.clone());

        let mut effective_fetch_params: HashMap<String, i32> = HashMap::new();
        for (name, opt_limit) in &all_interval_specs {
            let required_limit = opt_limit.unwrap_or(self.default_limit);
            effective_fetch_params
                .entry(name.clone())
                .and_modify(|current_limit| *current_limit = (*current_limit).max(required_limit))
                .or_insert(required_limit);
        }

        if effective_fetch_params.is_empty() {
            return Ok(HashMap::new());
        }

        println!(
            "Builder: Effective fetch params for {}: {:?}",
            self.pair_symbol, effective_fetch_params
        );

        let mut kline_data_map: HashMap<String, Vec<Kline>> = HashMap::new();

        // Fetch data one by one
        for (interval_name, &limit_to_use) in &effective_fetch_params {
            let interval = interval_name.clone();
            let pair_symbol_for_fetch = self.pair_symbol.to_string();

            println!(
                "Builder: Fetching data for {} interval {} with limit {}",
                pair_symbol_for_fetch, interval, limit_to_use
            );

            let kline_data: Vec<Kline> =
                fetch_binance_kline_usdt::<Kline>(&pair_symbol_for_fetch, &interval, limit_to_use)
                    .await
                    .with_context(|| {
                        format!(
                            "Builder: Failed fetching klines for {} interval {} with limit {}",
                            pair_symbol_for_fetch, interval, limit_to_use
                        )
                    })?;

            kline_data_map.insert(interval.clone(), kline_data);
        }

        println!(
            "Builder: Fetched kline data for intervals: {:?}",
            kline_data_map.keys()
        );
        Ok(kline_data_map)
    }

    // --- Formatting Sections ---

    /// Formats the Klines section based on intervals requested via `with_klines`.
    fn format_klines_section(
        &self,
        kline_data_map: &HashMap<String, Vec<Kline>>,
    ) -> Result<String> {
        if self.kline_intervals.is_empty() {
            return Ok(String::new());
        }

        let mut klines_output = String::new();
        klines_output.push_str("\n**Klines (Price History):**\n");

        let mut sorted_requested_klines = self.kline_intervals.clone();
        sorted_requested_klines.sort_by(|a, b| a.0.cmp(&b.0));

        for (interval_name, opt_limit) in &sorted_requested_klines {
            let display_interval = match opt_limit {
                Some(limit) => format!("{}:{}", interval_name, limit),
                None => interval_name.clone(),
            };

            if let Some(data) = kline_data_map.get(interval_name) {
                if data.is_empty() {
                    klines_output.push_str(&format!(" ({}) No data found.\n", display_interval));
                    continue;
                }
                match klines_to_csv(data) {
                    Ok(csv_data) => {
                        klines_output.push_str(&format!("\n* Price: {}\n", interval_name));
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
                            interval_name, e
                        );
                    }
                }
            } else {
                klines_output.push_str(&format!(
                    "\n* Interval: {} (Data unexpectedly missing after fetch)\n",
                    display_interval
                ));
                eprintln!(
                    "Warning: Kline data for interval {} requested via with_klines but not found in map.",
                    interval_name
                );
            }
        }
        Ok(klines_output)
    }

    /// Formats the Stochastic RSI section based on intervals requested via `with_stoch_rsi`.
    fn format_stoch_rsi_section(
        &self,
        kline_data_map: &HashMap<String, Vec<Kline>>,
    ) -> Result<String> {
        if self.stoch_rsi_intervals.is_empty() {
            return Ok(String::new());
        }

        let mut stoch_rsi_output = String::new();
        stoch_rsi_output.push_str("\n**Stochastic RSI:**\n");

        let mut sorted_requested_rsi = self.stoch_rsi_intervals.clone();
        sorted_requested_rsi.sort_by(|a, b| a.0.cmp(&b.0));

        for (interval_name, opt_limit) in &sorted_requested_rsi {
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
                match get_stoch_rsi_csv(data) {
                    Ok(stoch_rsi_csv) => {
                        stoch_rsi_output
                            .push_str(&format!("\n* Stochastic RSI: {}\n", interval_name));
                        stoch_rsi_output.push_str("```csv\n");
                        stoch_rsi_output.push_str(&stoch_rsi_csv);
                        stoch_rsi_output.push_str("```\n");
                    }
                    Err(e) => {
                        stoch_rsi_output.push_str(&format!(
                            "\n* Interval: {} (Error calculating StochRSI: {})\n",
                            display_interval, e
                        ));
                        eprintln!("Error calculating StochRSI for {}: {}", interval_name, e);
                    }
                }
            } else {
                stoch_rsi_output.push_str(&format!(
                    "\n* Interval: {} (Kline data unexpectedly missing for StochRSI calculation)\n",
                    display_interval
                ));
                eprintln!(
                    "Warning: Kline data for interval {} needed for StochRSI but not found in map.",
                    interval_name
                );
            }
        }
        Ok(stoch_rsi_output)
    }

    fn format_bb_section(&self, kline_data_map: &HashMap<String, Vec<Kline>>) -> Result<String> {
        if self.bb_intervals.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();
        output.push_str("\n**Boilinger Band:**\n");

        let mut sorted_requested_bb = self.bb_intervals.clone();
        sorted_requested_bb.sort_by(|a, b| a.0.cmp(&b.0));

        for (interval_name, opt_limit) in &sorted_requested_bb {
            let display_interval = match opt_limit {
                Some(limit) => format!("{}:{}", interval_name, limit),
                None => interval_name.clone(),
            };

            if let Some(data) = kline_data_map.get(interval_name) {
                if data.is_empty() {
                    output.push_str(&format!(
                        " ({}) No kline data available to calculate Boilinger Band.\n",
                        display_interval
                    ));
                    continue;
                }
                match get_latest_bb_ma(data) {
                    Ok(csv) => {
                        output.push_str(&format!("\n* Boilinger Band: {}\n", interval_name));
                        output.push_str("```csv\n");
                        output.push_str(&csv);
                        output.push_str("```\n");
                    }
                    Err(e) => {
                        output.push_str(&format!(
                            "\n* Interval: {} (Error calculating Boilinger Band: {})\n",
                            display_interval, e
                        ));
                        eprintln!(
                            "Error calculating Boilinger Band for {}: {}",
                            interval_name, e
                        );
                    }
                }
            } else {
                output.push_str(&format!(
                    "\n* Interval: {} (Boilinger Band data unexpectedly missing for Boilinger Band calculation)\n",
                    display_interval
                ));
                eprintln!(
                    "Warning: Boilinger Band data for interval {} needed for Boilinger Band but not found in map.",
                    interval_name
                );
            }
        }
        Ok(output)
    }

    fn format_latest_bb_ma_section(
        &self,
        kline_data_map: &HashMap<String, Vec<Kline>>,
    ) -> Result<String> {
        if self.latest_bb_ma_intervals.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();
        output.push_str("\n**Boilinger Band and Moving Average:**\n");

        let mut sorted_requested_bb_ma = self.latest_bb_ma_intervals.clone();
        sorted_requested_bb_ma.sort_by(|a, b| a.0.cmp(&b.0));

        for (interval_name, opt_limit) in &sorted_requested_bb_ma {
            let display_interval = match opt_limit {
                Some(limit) => format!("{}:{}", interval_name, limit),
                None => interval_name.clone(),
            };

            if let Some(data) = kline_data_map.get(interval_name) {
                if data.is_empty() {
                    output.push_str(&format!(
                        " ({}) No kline data available to calculate Boilinger Band and Moving Average.\n",
                        display_interval
                    ));
                    continue;
                }
                match get_latest_bb_ma(data) {
                    Ok(detail) => {
                        output.push_str(&format!(
                            "\n* Boilinger Band and Moving Average: {}\n",
                            interval_name
                        ));
                        output.push_str("```\n");
                        output.push_str(&detail);
                        output.push_str("\n```\n");
                    }
                    Err(e) => {
                        output.push_str(&format!(
                            "\n* Interval: {} (Error calculating Boilinger Band and Moving Average: {})\n",
                            display_interval, e
                        ));
                        eprintln!(
                            "Error calculating Boilinger Band and Moving Average for {}: {}",
                            interval_name, e
                        );
                    }
                }
            } else {
                output.push_str(&format!(
                        "\n* Interval: {} (Boilinger Band data unexpectedly missing for Boilinger Band calculation)\n",
                        display_interval
                    ));
                eprintln!(
                        "Warning: Boilinger Band data for interval {} needed for Boilinger Band but not found in map.",
                        interval_name
                    );
            }
        }
        Ok(output)
    }

    // --- Public API Method ---

    /// **Fetches required data and formats it into a single Markdown report string.**
    ///
    /// This method generates a string containing sections for Klines, Stochastic RSI,
    /// etc., based on what was requested via `.with_klines()`, `.with_stoch_rsi()`, etc.
    /// Each section contains data formatted as CSV within Markdown code blocks.
    pub async fn build(&self) -> Result<String> {
        // Renamed back to build()
        let mut output_string = String::new();

        let klines_requested = !self.kline_intervals.is_empty();
        let rsi_requested = !self.stoch_rsi_intervals.is_empty();
        let bb_requested = !self.bb_intervals.is_empty();
        let latest_bb_requested = !self.latest_bb_ma_intervals.is_empty();

        // Add checks for other indicators...
        let any_data_requested = klines_requested || rsi_requested || bb_requested; // || other_requested ...

        if !any_data_requested {
            output_string.push_str("No historical data intervals specified.\n");
            return Ok(output_string);
        }

        let kline_data_map = self.fetch_each_intervals().await?;

        if kline_data_map.is_empty() && any_data_requested {
            output_string
                .push_str("Warning: No kline data could be fetched for the requested intervals.\n");
            return Ok(output_string);
        } else if kline_data_map.is_empty() {
            // This case should ideally be caught by !any_data_requested check above,
            // but kept as a safeguard.
            output_string.push_str("No historical data intervals specified.\n");
            return Ok(output_string);
        }

        // Append formatted sections if they were requested
        if klines_requested {
            output_string.push_str(&self.format_klines_section(&kline_data_map)?);
        }

        if rsi_requested {
            output_string.push_str(&self.format_stoch_rsi_section(&kline_data_map)?);
        }

        if bb_requested {
            output_string.push_str(&self.format_bb_section(&kline_data_map)?);
        }

        println!("latest_bb_requested:{latest_bb_requested}");
        if latest_bb_requested {
            output_string.push_str(&self.format_latest_bb_ma_section(&kline_data_map)?);
        }

        Ok(output_string)
    }
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    // No longer need PriceHistory import or mock here
    use anyhow::Result;
    use tokio;

    // Helper to check CSV structure (header + data rows) in the report string
    fn check_report_csv_block(
        content: &str,
        interval_display: &str,
        expected_header: &str,
    ) -> bool {
        let block_start_marker = format!(
            "\n* Interval: {}\n```csv\n{}",
            interval_display, expected_header
        );
        if let Some(header_start) = content.find(&block_start_marker) {
            let data_start = header_start + block_start_marker.len();
            if let Some(block_end) = content[data_start..].find("\n```\n") {
                let csv_data = &content[data_start..data_start + block_end];
                // Check if there's *any* content after the header, indicating data rows
                return !csv_data.trim().is_empty();
            }
        }
        println!(
            "Failed to find CSV block for interval '{}' with header '{}'",
            interval_display,
            expected_header.trim()
        ); // Debug helper
        false
    }

    #[tokio::test]
    async fn test_build_basic_report() -> Result<()> {
        // Renamed test
        let pair_symbol = "SOL_USDT";
        let default_limit = 50;
        let kline_intervals = ["1h", "4h:60"];
        let stoch_rsi_intervals = ["1h"];

        let builder = PriceHistoryBuilder::new(pair_symbol, default_limit)
            .with_klines(&kline_intervals)
            .with_stoch_rsi(&stoch_rsi_intervals);

        let result_string = builder.build().await?; // Use build()
        println!(
            "--- Basic Report Test Output ---\n{}\n--- End ---",
            result_string
        );

        // Check Klines Section
        assert!(result_string.contains("\n**Klines (Price History):**\n"));
        let kline_header = "open_time,open,high,low,close,volume,close_time\n";
        assert!(
            check_report_csv_block(&result_string, "1h", kline_header),
            "1h kline block check failed"
        );
        assert!(
            check_report_csv_block(&result_string, "4h:60", kline_header),
            "4h:60 kline block check failed"
        );

        // Check RSI Section
        assert!(result_string.contains("\n**Stochastic RSI:**\n"));
        let rsi_header = "at,stoch_rsi_k,stoch_rsi_d\n";
        assert!(
            check_report_csv_block(&result_string, "1h", rsi_header),
            "1h RSI block check failed"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_build_klines_only_report() -> Result<()> {
        // Renamed test
        let pair_symbol = "BTC_USDT";
        let default_limit = 30;
        let kline_intervals = ["15m:20", "1h"];

        let builder =
            PriceHistoryBuilder::new(pair_symbol, default_limit).with_klines(&kline_intervals);

        let result_string = builder.build().await?; // Use build()
        println!(
            "--- Klines Only Report Test Output ---\n{}\n--- End ---",
            result_string
        );

        assert!(result_string.contains("\n**Klines (Price History):**\n"));
        assert!(!result_string.contains("\n**Stochastic RSI:**\n")); // Ensure RSI section is absent

        let kline_header = "open_time,open,high,low,close,volume,close_time\n";
        assert!(check_report_csv_block(
            &result_string,
            "15m:20",
            kline_header
        ));
        assert!(check_report_csv_block(&result_string, "1h", kline_header));

        Ok(())
    }

    #[tokio::test]
    async fn test_build_rsi_only_report() -> Result<()> {
        // Renamed test
        let pair_symbol = "ETH_USDT";
        let default_limit = 50;
        let stoch_rsi_intervals = ["1h:40", "4h"];

        let builder = PriceHistoryBuilder::new(pair_symbol, default_limit)
            .with_stoch_rsi(&stoch_rsi_intervals);

        let result_string = builder.build().await?; // Use build()
        println!(
            "--- RSI Only Report Test Output ---\n{}\n--- End ---",
            result_string
        );

        assert!(!result_string.contains("\n**Klines (Price History):**\n")); // Ensure Klines section is absent
        assert!(result_string.contains("\n**Stochastic RSI:**\n"));

        let rsi_header = "at,stoch_rsi_k,stoch_rsi_d\n";
        assert!(check_report_csv_block(&result_string, "1h:40", rsi_header));
        assert!(check_report_csv_block(&result_string, "4h", rsi_header));

        Ok(())
    }

    #[tokio::test]
    async fn test_build_no_requests() -> Result<()> {
        // Renamed test
        let pair_symbol = "ADA_USDT";
        let default_limit = 50;
        let builder = PriceHistoryBuilder::new(pair_symbol, default_limit); // No .with calls

        let result_string = builder.build().await?; // Use build()
        println!(
            "--- No Request Report Test Output ---\n{}\n--- End ---",
            result_string
        );

        assert_eq!(result_string, "No historical data intervals specified.\n");
        Ok(())
    }
}
