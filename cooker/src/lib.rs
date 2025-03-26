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
        maybe_timeframe: Option<String>,
    ) -> Result<Response> {
        let output_result = predict_with_gemini(
            &prediction_type,
            gemini_api_key.to_owned(),
            pair_symbol,
            orderbook_limit,
            maybe_wallet_address,
            maybe_timeframe,
            None,
            None,
            None,
        )
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
        // Endpoint: /api/v1/predict/:token/:timeframe
        .get_async(
            "/api/v1/predict/:token/:timeframe",
            |_req, ctx| async move {
                let pair_symbol = match ctx.param("token") {
                    Some(token) => token.to_owned(),
                    None => return Response::error("Bad Request - Missing Token", 400),
                };

                // Get timeframe
                let timeframe = ctx.param("timeframe");

                handle_prediction_request(
                    PredictionType::Graph,
                    gemini_api_key,
                    orderbook_limit,
                    pair_symbol,
                    None,
                    timeframe.cloned(),
                )
                .await
            },
        )
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
    prediction_type: &PredictionType,
    gemini_api_key: String,
    pair_symbol: String,
    orderbook_limit: i32,
    maybe_wallet_address: Option<String>,
    maybe_timeframe: Option<String>,
    maybe_images: Option<Vec<ImageData>>,
    maybe_prompt: Option<String>,
    maybe_trading_predictions: Option<Vec<RefinedTradingPrediction>>,
) -> anyhow::Result<String, String> {
    let gemini_model = if maybe_images.is_some() {
        println!("✨ Some images");
        GeminiModel::Gemini2Flash
    } else {
        GeminiModel::default()
    };

    let provider = GeminiProvider::new_v1beta(&gemini_api_key);
    let (token_symbol, _) = get_token_and_pair_symbol_usdt(&pair_symbol);

    // Get price
    // TODO: more oracle?
    let kline_data_1s = fetch_binance_kline_usdt::<ConciseKline>(&pair_symbol, "1s", 1)
        .await
        .expect("Failed to get price.");
    let current_price = kline_data_1s[0].close;

    // Get position from wallet_address if provided
    let maybe_preps_positions = match maybe_wallet_address {
        Some(wallet_address) => match get_preps_position(Some(wallet_address)).await {
            Ok(positions) => positions,
            Err(error) => return Err(format!("Error getting position: {:?}", error.to_string())),
        },
        None => None,
    };

    // Use provided timeframe or default to "4h"
    let timeframe = maybe_timeframe.unwrap_or_else(|| "4h".to_owned());

    let context = TradingContext {
        token_symbol,
        pair_symbol,
        timeframe,
        current_price,
        maybe_preps_positions,
        maybe_trading_predictions,
    };

    let prompt = get_binance_prompt(
        prediction_type,
        &gemini_model,
        context.clone(),
        orderbook_limit,
    )
    .await
    .map_err(|e| e.to_string())?;

    let prompt = if maybe_prompt.is_some() {
        prompt + "\n" + &maybe_prompt.unwrap_or_default()
    } else {
        prompt
    };

    // Use empty vec if no images provided
    let images = maybe_images.unwrap_or_default();

    match prediction_type {
        PredictionType::Trading => {
            let prediction_result =
                TradePredictor::<TradingPrediction>::new(&provider, &gemini_model, &prompt)
                    .with_context(context.clone())
                    .with_images(images)
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
                    .with_images(images)
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
        PredictionType::Rebalance => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        predict_with_gemini, predictions::prediction_types::PredictionType,
        providers::gemini::ImageData,
    };
    use base64::Engine;

    #[tokio::test]
    async fn test_trading_prediction_with_wallet() {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let pair_symbol = "SOL_USDC";
        let wallet_address = std::env::var("WALLET_ADDRESS").ok();

        let result = predict_with_gemini(
            &PredictionType::Trading,
            gemini_api_key,
            pair_symbol.to_string(),
            1000,
            wallet_address,
            None,
            None,
            None,
            None,
        )
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

        let result = predict_with_gemini(
            &PredictionType::Graph,
            gemini_api_key,
            pair_symbol.to_string(),
            1000,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
        println!(
            "{:#?}",
            serde_json::from_str::<serde_json::Value>(&result).unwrap()
        );
    }

    #[tokio::test]
    async fn test_prediction_with_image_and_timeframe() {
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

        // Call the prediction function with image and timeframe
        let result = predict_with_gemini(
            &PredictionType::Graph,
            gemini_api_key,
            pair_symbol.to_string(),
            1000,
            None,                   // No wallet address
            Some("1h".to_string()), // Custom timeframe
            Some(images),           // Pass the image data
            None,
            None,
        )
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
