use anyhow::Result;

use super::gemini::GeminiModel;

pub trait AiProvider {
    #[allow(unused)]
    async fn call_api<T: serde::de::DeserializeOwned + Send>(
        &self,
        model: &GeminiModel,
        prompt: &str,
        maybe_response_schema: Option<&str>,
    ) -> Result<T>;
}
