use crate::providers::gemini::{GeminiModel, GeminiProvider, ImageData};
use anyhow::Result;
use chrono_tz::Asia::Tokyo;
use common::{Refinable, TradingContext};
use md5;
use serde::Deserialize;

// Builder for predictions
pub struct TradePredictor<'a, T> {
    provider: &'a GeminiProvider,
    model: &'a GeminiModel,
    prompt: &'a str,
    context: Option<TradingContext>,
    images: Vec<ImageData>,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T> TradePredictor<'a, T>
where
    T: Refinable + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    pub fn new(provider: &'a GeminiProvider, model: &'a GeminiModel, prompt: &'a str) -> Self {
        Self {
            provider,
            model,
            prompt,
            context: None,
            images: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_context(mut self, context: TradingContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_images(mut self, images: Vec<ImageData>) -> Self {
        self.images = images;
        self
    }

    pub async fn run(self) -> Result<T::Refined> {
        let gemini_response: T = self
            .provider
            .call_api(self.model, self.prompt.to_string())
            .with_images(self.images)
            .run()
            .await?;

        let model_name = self.model.as_ref().to_string();
        // TOFIX: Use base prompt hash
        let prompt_hash = md5::compute(self.prompt)
            .iter()
            .fold(String::new(), |acc, b| format!("{acc}{b:02x}"));
        let refined_output = gemini_response.refine(Tokyo, &model_name, &prompt_hash, self.context);

        Ok(refined_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::gemini::{GeminiModel, GeminiProvider};
    use base64::Engine;
    use common::TradingPrediction;
    use tokio;

    // TODO: defined output format
    #[tokio::test]
    async fn test_get_prediction_with_no_context() -> Result<()> {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let provider = GeminiProvider::new_v1beta(&gemini_api_key);

        let model = GeminiModel::Gemini25Flash;
        let prompt = r#"Extract the number and technical analysis from provided trading graphs and validate the signals to proof that you understand the pictures as JSON."#;
        let image_bytes = std::fs::read("../feeder/test.png").expect("Failed to read test.png");
        let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
        let images = vec![ImageData {
            mime_type: "image/png".to_string(),
            data: base64_image,
        }];

        let result = TradePredictor::<TradingPrediction>::new(&provider, &model, prompt)
            .with_images(images)
            .run()
            .await?;
        println!("result: {result:#?}");

        Ok(())
    }

    // TODO: defined output format
    #[tokio::test]
    async fn test_get_prediction_with_no_context_multiples_image() -> Result<()> {
        dotenvy::from_filename(".env").expect("No .env file");

        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let provider = GeminiProvider::new_v1beta(&gemini_api_key);

        let model = GeminiModel::Gemini25Flash;
        let prompt = r#"Extract the number and technical analysis from provided trading graphs and validate the signals to proof that you understand the pictures as JSON.
            Must extract current_price_1h and current_price_4h."#;
        let image_bytes = std::fs::read("../feeder/test_1h.png").expect("Failed to read test.png");
        let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

        let image_bytes2 = std::fs::read("../feeder/test_4h.png").expect("Failed to read test.png");
        let base64_image2 = base64::engine::general_purpose::STANDARD.encode(&image_bytes2);
        let images = vec![
            ImageData {
                mime_type: "image/png".to_string(),
                data: base64_image,
            },
            ImageData {
                mime_type: "image/png".to_string(),
                data: base64_image2,
            },
        ];

        let result = TradePredictor::<TradingPrediction>::new(&provider, &model, prompt)
            .with_images(images)
            .run()
            .await?;
        println!("result: {result:#?}");

        Ok(())
    }
}
