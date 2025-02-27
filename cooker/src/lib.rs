use predictions::predict::get_prediction;
use providers::gemini::{GeminiModel, GeminiProvider};

mod predictions;
mod providers;
mod sources;
mod transforms;

use sources::jup::get_preps_position;

use serde::Deserialize;
use worker::*;

#[derive(Deserialize)]
struct SuggestQuery {
    wallet_address: String,
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let orderbook_limit: i32 = 600;
    let gemini_api_key = env
        .secret("GEMINI_API_KEY")
        .expect("Expect GEMINI_API_KEY")
        .to_string();

    let gemini_api_key = gemini_api_key.as_str();

    let router = Router::new();
    router
        .get_async("/suggest/:token", |req, ctx| async move {
            if let Some(pair_symbol) = ctx.param("token") {
                let maybe_wallet_address = match req.query::<SuggestQuery>() {
                    Ok(q) => Some(q.wallet_address),
                    Err(_e) => None,
                };

                let output_result = predict_with_gemini(
                    gemini_api_key.to_owned(),
                    pair_symbol.to_owned(),
                    orderbook_limit,
                    maybe_wallet_address,
                )
                .await;

                match output_result {
                    Ok(output) => {
                        let output_json_result: anyhow::Result<serde_json::Value, _> =
                            serde_json::from_str(&output);
                        match output_json_result {
                            Ok(output_json) => Response::from_json(&output_json),
                            Err(e) => Response::error(
                                format!("Failed to parse prediction JSON: {}", e),
                                500,
                            ),
                        }
                    }
                    Err(error_message) => {
                        Response::error(format!("Prediction failed: {}", error_message), 500)
                    }
                }
            } else {
                Response::error("Bad Request - Missing Token", 400)
            }
        })
        .run(req, env)
        .await
}

pub async fn predict_with_gemini(
    gemini_api_key: String,
    pair_symbol: String,
    orderbook_limit: i32,
    maybe_wallet_address: Option<String>,
) -> anyhow::Result<String, String> {
    let gemini_model = GeminiModel::default();
    let provider = GeminiProvider::new_v1beta(&gemini_api_key);

    // TODO: Over token/timeout for this one
    // Get position from wallet_address if has
    let maybe_preps_positions = match get_preps_position(maybe_wallet_address).await {
        Ok(positions) => positions,
        Err(error) => return Err(format!("Error getting position: {:?}", error)),
    };

    let prediction_result = get_prediction(
        &pair_symbol,
        &provider,
        &gemini_model,
        orderbook_limit,
        maybe_preps_positions,
    )
    .await;

    // TODO: return as json
    match prediction_result {
        Ok(prediction_output) => Ok(serde_json::to_string(&prediction_output)
            .map_err(|e| format!("Failed to serialize prediction output to JSON: {}", e))?),
        Err(error) => Err(format!("Error getting prediction: {:?}", error)),
    }
}

#[cfg(test)]
mod tests {
    use crate::predict_with_gemini;

    // #[ignore]
    #[tokio::test]
    async fn test() {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let symbol = "SOLUSDT";
        let wallet_address = std::env::var("WALLET_ADDRESS").ok();

        let result = predict_with_gemini(gemini_api_key, symbol.to_string(), 100, wallet_address)
            .await
            .unwrap();
        println!(
            "{:#?}",
            serde_json::from_str::<serde_json::Value>(&result).unwrap()
        );
    }
}
