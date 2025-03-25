use crate::providers::gemini::{GeminiModel, GeminiProvider, ImageData};
use anyhow::Result;
use chrono_tz::Asia::Tokyo;
use common::{Refinable, TradingContext};
use md5;
use serde::Deserialize;

// Builder for get_prediction
pub struct PredictionRequestBuilder<'a, T> {
    provider: &'a GeminiProvider,
    model: &'a GeminiModel,
    prompt: &'a str,
    context: Option<TradingContext>,
    images: Vec<ImageData>,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T> PredictionRequestBuilder<'a, T>
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

    pub async fn build(self) -> Result<T::Refined> {
        let gemini_response: T = self
            .provider
            .call_api::<T>(self.model, self.prompt.to_string())
            .with_images(self.images)
            .build()
            .await?;

        let model_name = self.model.as_ref().to_string();
        let prompt_hash = md5::compute(self.prompt)
            .iter()
            .fold(String::new(), |acc, b| format!("{acc}{:02x}", b));
        let refined_output = gemini_response.refine(Tokyo, &model_name, &prompt_hash, self.context);

        Ok(refined_output)
    }
}

pub fn get_prediction<'a, T>(
    provider: &'a GeminiProvider,
    model: &'a GeminiModel,
    prompt: &'a str,
) -> PredictionRequestBuilder<'a, T>
where
    T: Refinable + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    PredictionRequestBuilder::new(provider, model, prompt)
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
            "Extract the number and technical analysis from the trading graph and validate the signals to proof that you understand the picture.";
        let image_bytes = std::fs::read("../feeder/test.png").expect("Failed to read test.png");
        let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
        let images = vec![ImageData {
            mime_type: "image/png".to_string(),
            data: base64_image,
        }];

        let result = get_prediction::<TradingPrediction>(&provider, &model, prompt)
            .with_images(images)
            .build()
            .await?;
        println!("result: {result:#?}");

        Ok(())
    }
}
