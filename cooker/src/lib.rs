use predictions::predict::get_prediction;
use providers::gemini::{GeminiModel, GeminiProvider};

mod predictions;
mod providers;
mod sources;

use worker::*;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let limit: i32 = 100;
    let gemini_api_key = env
        .var("GEMINI_API_KEY")
        .expect("Expect GEMINI_API_KEY")
        .to_string();

    let gemini_api_key = gemini_api_key.as_str();

    let router = Router::new();
    router
        .get_async("/suggest/:token", |_req, ctx| async move {
            if let Some(symbol) = ctx.param("token") {
                let output_result =
                    predict_with_gemini(gemini_api_key.to_owned(), symbol.to_owned(), limit).await;

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
    symbol: String,
    limit: i32,
) -> anyhow::Result<String, String> {
    let gemini_model = GeminiModel::FlashLitePreview;
    let provider = GeminiProvider::new_v1beta(&gemini_api_key);

    let prediction_result = get_prediction(&symbol, &provider, &gemini_model, limit).await;

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
        let limit = 10;
        let result = predict_with_gemini(gemini_api_key, symbol.to_string(), limit)
            .await
            .unwrap();
        println!("{:#?}", result);
        println!(
            "{:#?}",
            serde_json::from_str::<serde_json::Value>(&result).unwrap()
        );
    }
}
