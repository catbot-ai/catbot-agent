use anyhow::Result;
use serde::Serialize;

use super::gemini::GeminiModel;

pub trait AiProvider {
    async fn call_api<T: serde::de::DeserializeOwned + Send>(
        &self,
        model: &GeminiModel,
        prompt: &str,
        maybe_response_schema: Option<&str>,
    ) -> Result<T>;
}

#[derive(Serialize, Default, Clone)]
pub struct PriceHistory {
    pub price_history_1m: Option<String>, // 1-minute candlestick data (optional)
    pub price_history_5m: Option<String>, // 5-minute candlestick data (optional)
    pub price_history_1h: Option<String>, // 1-hour candlestick data (optional)
    pub price_history_4h: Option<String>, // 4-hour candlestick data (optional)
    pub price_history_1d: Option<String>, // 1-day candlestick data (optional)
}

impl PriceHistory {
    /// Generates a formatted string for price history data, including only the timeframes that are present.
    pub fn to_formatted_string(&self) -> String {
        let mut price_history_string = String::new();

        macro_rules! push_history_if_some {
            ($field:ident, $timeframe:expr) => {
                if let Some(data) = &self.$field {
                    price_history_string.push_str(&format!(
                        "**Price History ({} timeframe):**\n{}\n",
                        $timeframe, data
                    ));
                }
            };
        }

        // Use the macro to reduce repetition
        push_history_if_some!(price_history_1m, "1m");
        push_history_if_some!(price_history_5m, "5m");
        push_history_if_some!(price_history_1h, "1h");
        push_history_if_some!(price_history_4h, "4h");
        push_history_if_some!(price_history_1d, "1d");

        price_history_string
    }
}
