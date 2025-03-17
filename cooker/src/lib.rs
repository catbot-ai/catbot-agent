use predictions::{
    binance::get_binance_prompt, predict::get_prediction, prediction_types::PredictionType,
};
use providers::gemini::{GeminiModel, GeminiProvider};

mod predictions;
mod providers;

use common::{jup::get_preps_position, GraphPredictionOutput, PredictionOutput, SuggestionOutput};
use worker::*;

pub enum Route {
    SUGGESTIONS,
    PREDICTIONS,
}

pub async fn handle_root(_req: Request, _ctx: RouteContext<()>) -> worker::Result<Response> {
    Response::from_html(
        r#"<a href="/api/v1/suggest/SOL_USDT/1h">SUGGEST</a><br><a href="/api/v1/predict/SOL_USDT/1h">PREDICT</a><br>"#,
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
        route: Route,
        gemini_api_key: &str,
        orderbook_limit: i32,
        pair_symbol: String,
        maybe_wallet_address: Option<String>,
        maybe_timeframe: Option<String>,
    ) -> Result<Response> {
        let output_result = match route {
            Route::SUGGESTIONS => {
                predict_with_gemini(
                    &PredictionType::Suggestions,
                    gemini_api_key.to_owned(),
                    pair_symbol,
                    orderbook_limit,
                    maybe_wallet_address,
                    maybe_timeframe,
                )
                .await
            }
            Route::PREDICTIONS => {
                predict_with_gemini(
                    &PredictionType::GraphPredictions,
                    gemini_api_key.to_owned(),
                    pair_symbol,
                    orderbook_limit,
                    maybe_wallet_address,
                    maybe_timeframe,
                )
                .await
            }
        };

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
                    Route::SUGGESTIONS,
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
                Route::SUGGESTIONS,
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
                    Route::PREDICTIONS,
                    gemini_api_key,
                    orderbook_limit,
                    pair_symbol,
                    None,
                    timeframe.cloned(),
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
) -> anyhow::Result<String, String> {
    let gemini_model = GeminiModel::default();
    let provider = GeminiProvider::new_v1beta(&gemini_api_key);

    // Get position from wallet_address if provided
    let maybe_preps_positions = match maybe_wallet_address {
        Some(wallet_address) => match get_preps_position(Some(wallet_address)).await {
            Ok(positions) => positions,
            Err(error) => return Err(format!("Error getting position: {:?}", error.to_string())),
        },
        None => None,
    };

    let prompt = get_binance_prompt(
        prediction_type,
        &pair_symbol,
        &gemini_model,
        orderbook_limit,
        maybe_preps_positions,
        maybe_timeframe,
    )
    .await
    .map_err(|e| e.to_string())?;

    let prediction_result = match prediction_type {
        PredictionType::Suggestions => {
            get_prediction::<SuggestionOutput>(&provider, &gemini_model, prompt)
                .await
                .map(PredictionOutput::Suggestions)
                .map_err(|e| format!("\nError getting suggestion prediction: {e}"))
        }
        PredictionType::GraphPredictions => {
            get_prediction::<GraphPredictionOutput>(&provider, &gemini_model, prompt)
                .await
                .map(PredictionOutput::GraphPredictions)
                .map_err(|e| format!("\nError getting graph prediction: {e}"))
        }
    };

    match prediction_result {
        Ok(prediction_output) => Ok(serde_json::to_string_pretty(&prediction_output)
            .map_err(|e| format!("Failed to serialize prediction output to JSON: {}", e))?),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use crate::{predict_with_gemini, predictions::prediction_types::PredictionType};

    #[tokio::test]
    async fn test_with_wallet() {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let pair_symbol = "SOL_USDT";
        let wallet_address = std::env::var("WALLET_ADDRESS").ok();

        let result = predict_with_gemini(
            &PredictionType::Suggestions,
            gemini_api_key,
            pair_symbol.to_string(),
            100,
            wallet_address,
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
    async fn test_without_wallet() {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let pair_symbol = "SOL_USDT";

        let result = predict_with_gemini(
            &PredictionType::Suggestions,
            gemini_api_key,
            pair_symbol.to_string(),
            100,
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
    async fn test_prediction_without_wallet() {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let pair_symbol = "SOL_USDT";

        let result = predict_with_gemini(
            &PredictionType::GraphPredictions,
            gemini_api_key,
            pair_symbol.to_string(),
            100,
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
}
