use crate::providers::{
    core::AiProvider,
    gemini::{GeminiModel, GeminiProvider},
};
use chrono_tz::Asia::Tokyo;

use anyhow::Result;
use common::{Refinable, TradingContext};
use md5;
use serde::Deserialize;

pub async fn get_prediction<T>(
    provider: &GeminiProvider,
    model: &GeminiModel,
    prompt: String,
    context: TradingContext,
) -> Result<T::Refined>
where
    T: Refinable + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    println!("\nCalling Gemini API...");
    let gemini_response = provider.call_api::<T>(model, &prompt, None).await?;

    let model_name = model.as_ref().to_string();
    let prompt_hash = md5::compute(prompt)
        .iter()
        .fold(String::new(), |acc, b| format!("{acc}{:02x}", b));
    let refined_output = gemini_response.refine(Tokyo, &model_name, &prompt_hash, context);

    Ok(refined_output)
}
