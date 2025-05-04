use predictions::{
    binance::get_binance_prompt, predict::TradePredictor, prediction_types::PredictionType,
};
use providers::gemini::{GeminiModel, GeminiProvider, ImageData};

mod predictions;
mod providers;

use common::{
    binance::{fetch_binance_kline_usdt, get_token_and_pair_symbol_usdt},
    jup::get_preps_position,
    ConciseKline, GraphPrediction, RefinedTradingPrediction, TradingContext, TradingPrediction,
};
use worker::*;

pub async fn handle_root(_req: Request, _ctx: RouteContext<()>) -> worker::Result<Response> {
    Response::from_html(
        r#"<a href="/api/v1/suggest/SOL_USDT">SUGGEST</a><br><a href="/api/v1/predict/SOL_USDT/1h">PREDICT</a><br>"#,
    )
}

// --- Builder Pattern Implementation ---

#[derive(Clone, Debug)]
pub struct PredictionRequest {
    prediction_type: PredictionType,
    gemini_api_key: String,
    pair_symbol: String,
    orderbook_limit: i32,
    wallet_address: Option<String>,
    interval: Option<String>,
    images: Option<Vec<ImageData>>,
    prompt: Option<String>,
    trading_predictions: Option<Vec<RefinedTradingPrediction>>,
    kline_intervals: Option<Vec<String>>,
    stoch_rsi_intervals: Option<Vec<String>>,
    latest_bb_ma_intervals: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct PredictionRequestBuilder {
    request: PredictionRequest,
}

impl PredictionRequestBuilder {
    pub fn new(
        prediction_type: PredictionType,
        gemini_api_key: String,
        pair_symbol: String,
        orderbook_limit: i32,
    ) -> Self {
        Self {
            request: PredictionRequest {
                prediction_type,
                gemini_api_key,
                pair_symbol,
                orderbook_limit,
                wallet_address: None,
                interval: None,
                images: None,
                prompt: None,
                trading_predictions: None,
                kline_intervals: None,
                stoch_rsi_intervals: None,
                latest_bb_ma_intervals: None,
            },
        }
    }

    pub fn wallet_address(mut self, wallet_address: Option<String>) -> Self {
        self.request.wallet_address = wallet_address;
        self
    }

    pub fn interval(mut self, interval: Option<String>) -> Self {
        self.request.interval = interval;
        self
    }

    pub fn images(mut self, images: Option<Vec<ImageData>>) -> Self {
        self.request.images = images;
        self
    }

    pub fn prompt(mut self, prompt: Option<String>) -> Self {
        self.request.prompt = prompt;
        self
    }

    pub fn trading_predictions(
        mut self,
        predictions: Option<Vec<RefinedTradingPrediction>>,
    ) -> Self {
        self.request.trading_predictions = predictions;
        self
    }

    pub fn kline_intervals(mut self, intervals: Option<Vec<String>>) -> Self {
        self.request.kline_intervals = intervals;
        self
    }

    pub fn stoch_rsi_intervals(mut self, intervals: Option<Vec<String>>) -> Self {
        self.request.stoch_rsi_intervals = intervals;
        self
    }

    pub fn latest_bb_ma_intervals(mut self, intervals: Option<Vec<String>>) -> Self {
        self.request.latest_bb_ma_intervals = intervals;
        self
    }

    pub async fn predict(self) -> anyhow::Result<String, String> {
        predict_with_gemini(self.request).await
    }
}

// --- End Builder Pattern Implementation ---

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let orderbook_limit: i32 = 1000;
    let gemini_api_key = env
        .secret("GEMINI_API_KEY")
        .expect("Expect GEMINI_API_KEY")
        .to_string();

    let gemini_api_key = gemini_api_key.as_str();

    let router = Router::new();

    // Shared handler logic
    async fn handle_prediction_request(
        prediction_type: PredictionType,
        gemini_api_key: &str,
        orderbook_limit: i32,
        pair_symbol: String,
        maybe_wallet_address: Option<String>,
        maybe_interval: Option<String>,
    ) -> Result<Response> {
        let output_result = PredictionRequestBuilder::new(
            prediction_type, // Pass prediction_type directly
            gemini_api_key.to_owned(),
            pair_symbol,
            orderbook_limit,
        )
        .wallet_address(maybe_wallet_address)
        .interval(maybe_interval)
        // Other fields default to None
        .predict() // Call predict on the builder
        .await;

        match output_result {
            Ok(output) => match serde_json::from_str::<serde_json::Value>(&output) {
                Ok(output_json) => Response::from_json(&output_json),
                Err(e) => Response::error(format!("Failed to parse prediction JSON: {}", e), 500),
            },
            Err(error_message) => {
                Response::error(format!("Prediction failed: {}", error_message), 500)
            }
        }
    }

    router
        .get_async("/", handle_root)
        // Endpoint: /api/v1/suggest/:token/:wallet_address
        .get_async(
            "/api/v1/suggest/:token/:wallet_address",
            |_req, ctx| async move {
                let pair_symbol = match ctx.param("token") {
                    Some(token) => token.to_owned(),
                    None => return Response::error("Bad Request - Missing Token", 400),
                };
                let maybe_wallet_address = ctx.param("wallet_address").cloned();
                handle_prediction_request(
                    PredictionType::Trading,
                    gemini_api_key,
                    orderbook_limit,
                    pair_symbol,
                    maybe_wallet_address,
                    None,
                )
                .await
            },
        )
        // Endpoint: /api/v1/suggest/:token
        .get_async("/api/v1/suggest/:token", |_req, ctx| async move {
            let pair_symbol = match ctx.param("token") {
                Some(token) => token.to_owned(),
                None => return Response::error("Bad Request - Missing Token", 400),
            };
            handle_prediction_request(
                PredictionType::Trading,
                gemini_api_key,
                orderbook_limit,
                pair_symbol,
                None,
                None,
            )
            .await
        })
        // Endpoint: /api/v1/predict/:token/:interval
        .get_async("/api/v1/predict/:token/:interval", |_req, ctx| async move {
            let pair_symbol = match ctx.param("token") {
                Some(token) => token.to_owned(),
                None => return Response::error("Bad Request - Missing Token", 400),
            };

            // Get interval
            let interval = ctx.param("interval");

            handle_prediction_request(
                PredictionType::Graph,
                gemini_api_key,
                orderbook_limit,
                pair_symbol,
                None,
                interval.cloned(),
            )
            .await
        })
        // Endpoint: /api/v1/rebalance/:token/:wallet_address",
        .get_async(
            "/api/v1/rebalance/:token/:wallet_address",
            |_req, ctx| async move {
                let pair_symbol = match ctx.param("token") {
                    Some(token) => token.to_owned(),
                    None => return Response::error("Bad Request - Missing Token", 400),
                };
                let maybe_wallet_address = ctx.param("wallet_address").cloned();
                handle_prediction_request(
                    PredictionType::Rebalance,
                    gemini_api_key,
                    orderbook_limit,
                    pair_symbol,
                    maybe_wallet_address,
                    None,
                )
                .await
            },
        )
        .run(req, env)
        .await
}

pub async fn predict_with_gemini(
    // Accept the PredictionRequest struct directly
    request: PredictionRequest,
) -> anyhow::Result<String, String> {
    // Access fields from the request struct
    let gemini_model = if request.images.is_some() {
        println!("✨ Some images");
        GeminiModel::Gemini2Flash
    } else {
        GeminiModel::default()
    };

    let provider = GeminiProvider::new_v1beta(&request.gemini_api_key);
    let (token_symbol, _) = get_token_and_pair_symbol_usdt(&request.pair_symbol);

    // Get price
    // TODO: more oracle?
    let kline_data_1s = fetch_binance_kline_usdt::<ConciseKline>(&request.pair_symbol, "1s", 1)
        .await
        .expect("Failed to get price.");
    let current_price = kline_data_1s[0].close;

    // Get position from wallet_address if provided
    let maybe_preps_positions = match &request.wallet_address {
        // Borrow request.wallet_address
        Some(wallet_address) => match get_preps_position(Some(wallet_address.clone())).await {
            // Clone wallet_address if needed
            Ok(positions) => positions,
            Err(error) => return Err(format!("Error getting position: {:?}", error.to_string())),
        },
        None => None,
    };

    // Use provided intervals and fallback from request
    let kline_intervals = request.kline_intervals.unwrap_or(
        vec!["15m:336", "1h:168", "4h:84", "1d:100"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    );

    // Include RSI or other indicators if desired in the report
    let stoch_rsi_intervals = request.stoch_rsi_intervals.unwrap_or(
        vec!["1h:168", "4h:84"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    );

    let latest_bb_ma_intervals = request.latest_bb_ma_intervals.unwrap_or(
        vec!["15m:336", "1h:168", "4h:84", "1d:100"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    );

    // Use provided interval or default to "4h" from request
    let interval = request.interval.unwrap_or_else(|| "4h".to_owned());

    let context = TradingContext {
        token_symbol,
        pair_symbol: request.pair_symbol, // Move pair_symbol from request
        interval,
        current_price,
        maybe_preps_positions,
        maybe_trading_predictions: request.trading_predictions, // Move trading_predictions from request
        kline_intervals,
        stoch_rsi_intervals,
        latest_bb_ma_intervals,
    };

    // Use request fields for get_binance_prompt
    let base_prompt = get_binance_prompt(
        &request.prediction_type,
        &gemini_model,
        context.clone(),
        request.orderbook_limit,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Handle optional additional prompt from request
    let prompt = if let Some(extra_prompt) = request.prompt {
        // Move prompt from request
        base_prompt + "\n" + &extra_prompt
    } else {
        base_prompt
    };

    // Use empty vec if no images provided from request
    let images = request.images.unwrap_or_default(); // Move images from request

    // Use request.prediction_type for matching
    match request.prediction_type {
        PredictionType::Trading => {
            let prediction_result =
                TradePredictor::<TradingPrediction>::new(&provider, &gemini_model, &prompt)
                    .with_context(context.clone())
                    .with_images(images) // Pass moved images
                    .run()
                    .await;

            match prediction_result {
                Ok(prediction_output) => Ok(serde_json::to_string_pretty(&prediction_output)
                    .map_err(|e| {
                        format!("Failed to serialize prediction output to JSON: {}", e)
                    })?),
                Err(error) => Err(error.to_string()),
            }
        }
        PredictionType::Graph => {
            let prediction_result =
                TradePredictor::<GraphPrediction>::new(&provider, &gemini_model, &prompt)
                    .with_context(context.clone())
                    .with_images(images) // Pass moved images
                    .run()
                    .await;

            match prediction_result {
                Ok(prediction_output) => Ok(serde_json::to_string_pretty(&prediction_output)
                    .map_err(|e| {
                        format!("Failed to serialize prediction output to JSON: {}", e)
                    })?),
                Err(error) => Err(error.to_string()),
            }
        }
        PredictionType::Rebalance => todo!("Rebalance prediction not yet implemented"), // Updated todo! message
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        predictions::prediction_types::PredictionType, providers::gemini::ImageData,
        PredictionRequestBuilder,
    };
    use base64::Engine;

    #[tokio::test]
    async fn test_trading_prediction_with_wallet() {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let pair_symbol = "SOL_USDC";
        let wallet_address = std::env::var("WALLET_ADDRESS").ok();

        // Use the builder
        let result = PredictionRequestBuilder::new(
            PredictionType::Trading,
            gemini_api_key,
            pair_symbol.to_string(),
            1000,
        )
        .wallet_address(wallet_address) // Set wallet address if available
        .predict() // Call predict
        .await
        .unwrap();
        println!(
            "{:#?}",
            serde_json::from_str::<serde_json::Value>(&result).unwrap()
        );
    }

    #[tokio::test]
    async fn test_graph_prediction_without_wallet() {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let pair_symbol = "SOL_USDT";

        // Use the builder
        let result = PredictionRequestBuilder::new(
            PredictionType::Graph,
            gemini_api_key,
            pair_symbol.to_string(),
            1000,
        )
        // No optional fields set here
        .predict() // Call predict
        .await
        .unwrap();
        println!(
            "{:#?}",
            serde_json::from_str::<serde_json::Value>(&result).unwrap()
        );
    }

    #[tokio::test]
    async fn test_prediction_with_image_and_interval() {
        // Load environment variables from .env file
        dotenvy::from_filename(".env").expect("No .env file found");

        // Retrieve Gemini API key from environment
        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");

        // Define trading pair symbol
        let pair_symbol = "SOL_USDT";

        // Load and encode test.png file
        let image_bytes = std::fs::read("../feeder/test.png").expect("Failed to read test.png");
        let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
        let images = vec![ImageData {
            mime_type: "image/png".to_string(),
            data: base64_image,
        }];

        // Call the prediction function using the builder
        let result = PredictionRequestBuilder::new(
            PredictionType::Graph,
            gemini_api_key,
            pair_symbol.to_string(),
            1000,
        )
        .interval(Some("1h".to_string())) // Set custom interval
        .images(Some(images)) // Set image data
        .predict() // Call predict
        .await;

        // Handle the result
        match result {
            Ok(json_string) => {
                let parsed_result: serde_json::Value =
                    serde_json::from_str(&json_string).expect("Failed to parse result as JSON");
                println!("{:#?}", parsed_result);
            }
            Err(error) => panic!("Prediction failed: {}", error),
        }
    }
}
