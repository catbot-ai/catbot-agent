use worker::*;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Deserialize, Serialize)]
struct SubscribeRequest {
    api_url: String,
    api_key: String,
    webhook_url: String,
    webhook_key: String,
}

async fn handle_subscribe(req: Request) -> Result<Response> {
    if req.method() != Method::Post {
        return Response::error("Method Not Allowed", 405);
    }

    let req_json = req.json::<SubscribeRequest>().await.map_err(|_| {
        Error::from_str("Bad Request: Invalid JSON for subscribe request")
    })?;

    // --- Call Feeder's /subscribe endpoint ---
    let feeder_url = format!("{}/subscribe", req_json.api_url); // Assuming feeder exposes /subscribe
    let client = reqwest::Client::new();
    let feeder_response = client.post(&feeder_url)
        .json(&req_json) // Forward the same request data to feeder
        .send()
        .await
        .map_err(|e| Error::from_str(&format!("Failed to call feeder service: {}", e)))?;

    if feeder_response.status().is_success() {
        Response::ok("Subscription request forwarded to feeder")
    } else {
        Response::error(format!("Feeder service error: {}", feeder_response.status()), feeder_response.status().as_u16())
    }
}


#[worker_entry]
pub async fn main(_req: Request, _env: Env, _ctx: RouteContext<()>) -> Result<Response> {
    let router = Router::new();

    router
        .post("/subscribe", handle_subscribe)
        .run(_req, _env, _ctx)
        .await
}

#[cfg(test)]
mod tests {
    // You can add consumer-specific tests here if needed.
}