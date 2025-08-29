use crate::sources::cooker::clean_json_string;
use anyhow::{anyhow, Context, Result};
use serde::de::DeserializeOwned;
use std::time::Duration; // Import Duration
use worker::{Fetcher, HttpRequest, Request, Response};

// Default values
const DEFAULT_RETRY_ATTEMPTS: usize = 2;
const DEFAULT_RETRY_DELAY_MS: u64 = 200; // Simple fixed delay for example
const _DEFAULT_TIMEOUT: Duration = Duration::from_secs(60); // Example default, not enforced yet

/// Helper struct for making calls to Cloudflare Worker service bindings using a builder pattern.
pub struct ServiceBinding {
    fetcher: Fetcher,
    request: Option<Request>,
    retry_attempts: usize,
    _timeout: Option<Duration>, // Field for timeout (currently informational)
}

impl ServiceBinding {
    /// Creates a new ServiceBinding helper with the given Fetcher and default settings.
    pub fn new(fetcher: Fetcher) -> Self {
        ServiceBinding {
            fetcher,
            request: None,
            retry_attempts: DEFAULT_RETRY_ATTEMPTS,
            _timeout: None, // Initialize timeout field
        }
    }

    /// Sets the original Request object to use for deriving the base URL.
    /// This is required before calling `fetch`.
    pub fn with_request(mut self, req: Request) -> Self {
        self.request = Some(req);
        self
    }

    /// Sets the number of retry attempts if the fetch fails.
    /// Defaults to `DEFAULT_RETRY_ATTEMPTS` (2). `0` means no retries.
    pub fn with_retry(mut self, attempts: usize) -> Self {
        self.retry_attempts = attempts;
        self
    }

    /// Sets a timeout duration for the fetch operation.
    /// **Note:** Due to WASM environment limitations, this timeout is not actively enforced
    /// by this helper's `fetch` method at this time. The actual timeout relies on the
    /// underlying Cloudflare platform configuration for service bindings.
    /// Defaults to `DEFAULT_TIMEOUT` (60 seconds) conceptually.
    pub fn with_timeout(self, _duration: Duration) -> Self {
        // self.timeout = Some(duration); // Store if needed for future implementation
        // No-op for now regarding active enforcement
        self
    }

    /// Executes the fetch call to the specified relative path on the bound service,
    /// with configured retries.
    ///
    /// Deserializes the JSON response into the specified type `T`.
    pub async fn fetch<T: DeserializeOwned>(self, relative_path: &str) -> Result<T> {
        // Use as_ref() to borrow the request instead of moving it from self
        let original_req = self
            .request
            .as_ref() // Borrow the Option's content
            .ok_or_else(|| anyhow!("Original request was not provided using with_request()"))?;

        let mut last_error: Option<anyhow::Error> = None;

        // Retry loop: 0..=self.retry_attempts means initial attempt + number of retries
        for attempt in 0..=self.retry_attempts {
            // Clone the borrowed request for each attempt as try_into() consumes it
            // Note: clone() returns a Result, hence the map_err
            let req_clone = original_req
                .clone() // Clone the borrowed &worker::Request
                .map_err(|e| anyhow!("Failed to clone request for attempt {}: {}", attempt, e))?;

            // --- Add delay before retrying (skip delay for the first attempt) ---
            if attempt > 0 {
                let delay_ms = DEFAULT_RETRY_DELAY_MS; // Could use exponential backoff here
                                                       // Placeholder for async sleep in WASM environment
                                                       // Needs a crate like `gloo-timers` or similar:
                                                       // gloo_timers::future::sleep(Duration::from_millis(delay_ms)).await;
                                                       // In a real Cloudflare Worker, you'd likely use `wasm-bindgen-futures`
                                                       // and `js-sys` to call `setTimeout` via `gloo_timers::future::TimeoutFuture`.
                                                       // For simplicity, we just log here.
                worker::console_log!(
                    "Retrying fetch (attempt {}/{}) after {}ms delay...",
                    attempt,
                    self.retry_attempts,
                    delay_ms
                );
                // Actual async delay would go here if implemented
                // e.g., using gloo_timers::future::sleep(Duration::from_millis(delay_ms)).await;
            }

            // --- Perform the fetch attempt ---
            // Borrowing self here is fine now because self.request was only borrowed above, not moved.
            let result = self.try_fetch_once::<T>(&req_clone, relative_path).await;

            match result {
                Ok(data) => return Ok(data), // Success, return immediately
                Err(e) => {
                    worker::console_error!("Fetch attempt {} failed: {}", attempt, e);
                    last_error = Some(e); // Store the error and continue to the next retry
                }
            }
        }

        // If all retries failed, return the last error encountered
        Err(last_error.unwrap_or_else(|| {
            anyhow!(
                "Service binding fetch failed after {} retries with no specific error recorded.",
                self.retry_attempts
            )
        }))
    }

    // Helper function encapsulating a single fetch attempt
    // Helper function encapsulating a single fetch attempt
    async fn try_fetch_once<T: DeserializeOwned>(
        &self,
        req: &Request, // Borrow the cloned request for this attempt
        relative_path: &str,
    ) -> Result<T> {
        // Clone the borrowed request to get an owned Request needed for try_into()
        let owned_req = req
            .clone()
            .map_err(|e| anyhow!("Failed to clone request within try_fetch_once: {}", e))?;

        // Convert the owned request to HttpRequest to modify its URI
        let mut http_request: HttpRequest = owned_req
            .try_into()
            .context("Failed to convert original Request to HttpRequest")?;

        // Get the original URI parts
        let original_uri = http_request.uri();
        let scheme = original_uri.scheme_str().unwrap_or("https");
        let authority = original_uri.authority().ok_or_else(|| {
            anyhow!("No authority found in original request URI needed for service binding call")
        })?;

        // Construct the new URI for the target service path
        let path_to_append = relative_path.trim_start_matches('/');
        let new_uri_str = format!("{scheme}://{authority}/{path_to_append}");

        // Update the HttpRequest URI
        *http_request.uri_mut() = new_uri_str
            .parse()
            .with_context(|| format!("Failed to parse new service binding URI: {new_uri_str}"))?;

        // Fetch the request from the target service using the fetcher
        let fetcher_response = self
            .fetcher
            .fetch_request(http_request)
            .await
            .map_err(|e| anyhow!("Service binding fetcher.fetch_request failed: {}", e))?;

        // Convert back to worker::Response to read the body
        let mut cf_response: Response = fetcher_response
            .try_into()
            .context("Failed to convert FetcherResponse to worker::Response")?;

        // Check if the underlying response status is successful before reading body
        if !(cf_response.status_code() >= 200 && cf_response.status_code() < 300) {
            let status = cf_response.status_code();
            let body_text = cf_response
                .text()
                .await
                .unwrap_or_else(|_| "[failed to read error body]".to_string());
            return Err(anyhow!(
                "Service binding fetch returned non-success status: {}. Body: {}",
                status,
                body_text
            ));
        }

        let response_text = cf_response
            .text()
            .await
            .map_err(|e| anyhow!("Failed to read service binding response text: {}", e))?; // Convert worker::Error

        // Deserialize the JSON response text
        let cleaned_response_text = clean_json_string(&response_text);
        serde_json::from_str(cleaned_response_text).with_context(|| {
            format!(
                "Failed to deserialize service binding response into {}. Original text: '{}'",
                std::any::type_name::<T>(),
                response_text
            )
        })
    }
}
