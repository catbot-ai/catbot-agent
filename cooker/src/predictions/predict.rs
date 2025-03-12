use crate::providers::{
    core::AiProvider,
    gemini::{GeminiModel, GeminiProvider},
};
use chrono_tz::Asia::Tokyo;
use common::{PredictionOutput, PredictionOutputWithTimeStampBuilder, RefinedPredictionOutput};

use anyhow::Result;
use md5;

pub async fn get_prediction(
    provider: &GeminiProvider,
    model: &GeminiModel,
    prompt: String,
) -> Result<RefinedPredictionOutput> {
    // --- Call Gemini API ---
    println!("Calling Gemini API...");
    let gemini_response = provider
        .call_api::<PredictionOutput>(model, &prompt, None)
        .await?;

    let model_name = model.as_ref().to_string();
    let prompt_hash = format!("{:x}", md5::compute(prompt));
    let prediction_output_with_timestamp =
        PredictionOutputWithTimeStampBuilder::new(gemini_response, Tokyo)
            .build(&model_name, &prompt_hash);

    Ok(prediction_output_with_timestamp)
}
