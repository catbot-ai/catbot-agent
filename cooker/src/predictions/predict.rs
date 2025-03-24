use crate::providers::gemini::{GeminiModel, GeminiProvider, ImageData};
use chrono_tz::Asia::Tokyo;

use anyhow::Result;
use common::{Refinable, TradingContext};
use md5;
use serde::Deserialize;

pub async fn get_prediction<T>(
    provider: &GeminiProvider,
    model: &GeminiModel,
    prompt: String,
    context: Option<TradingContext>,
    images: Vec<ImageData>,
) -> Result<T::Refined>
where
    T: Refinable + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    let gemini_response = provider
        .call_api_with_images::<T>(model, &prompt, images, None)
        .await?;

    let model_name = model.as_ref().to_string();
    let prompt_hash = md5::compute(&prompt)
        .iter()
        .fold(String::new(), |acc, b| format!("{acc}{:02x}", b));
    let refined_output = gemini_response.refine(Tokyo, &model_name, &prompt_hash, context);

    Ok(refined_output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::gemini::{GeminiModel, GeminiProvider};
    use base64::Engine;
    use common::TradingPrediction;
    use tokio;

    #[tokio::test]
    async fn test_get_prediction_with_no_context() -> Result<()> {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let provider = GeminiProvider::new_v1beta(&gemini_api_key);

        let model = GeminiModel::Gemini2Flash;
        let prompt =
            "Extract the number and technical analysis from the trading graph and validate the signals to proof that you understand the picture.".to_string();
        let image_bytes = std::fs::read("../feeder/test.png").expect("Failed to read test.png");
        let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
        let images = vec![ImageData {
            mime_type: "image/png".to_string(),
            data: base64_image,
        }];

        // Call get_prediction with context: None
        let result =
            get_prediction::<TradingPrediction>(&provider, &model, prompt.clone(), None, images)
                .await?;
        println!("result:{result:#?}");

        Ok(())
    }
}
