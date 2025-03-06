mod charts;

use charts::candle::{draw_candle, ChartMetaData};
use std::ops::Deref;
use worker::*;

// TODO: call service binding
async fn gen_candle(
    _pair_symbol: String,
    _timeframe: String,
) -> Vec<(&'static str, f32, f32, f32, f32)> {
    vec![
        ("2019-04-25", 130.06, 131.37, 128.83, 129.15),
        ("2019-04-24", 125.79, 125.85, 124.52, 125.01),
    ]
}

// TODO: pixel font
const DEFAULT_FONT_NAME: &str = "Roboto-Light.ttf";

pub async fn handle_chart(_: Request, ctx: RouteContext<()>) -> worker::Result<Response> {
    if let Some(pair_symbol) = ctx.param("token") {
        // Get timeframe
        let binding = "5m".to_string();
        let timeframe = ctx.param("timeframe").unwrap_or(&binding);

        // Get font
        let kv_store = ctx.kv("ASSETS").unwrap();
        let font = kv_store
            .get(DEFAULT_FONT_NAME)
            .bytes()
            .await
            .unwrap()
            .unwrap();

        // Get data
        let candle_data = gen_candle(
            pair_symbol.deref().to_string(),
            timeframe.deref().to_string(),
        )
        .await;

        // Get chart metadata
        let chart_metadata = ChartMetaData {
            title: format!("{pair_symbol} {timeframe}"),
        };

        // Get image
        let buffer = draw_candle(font, chart_metadata, candle_data).unwrap();

        let mut headers = Headers::new();
        headers.set("content-type", "image/png")?;
        let response = Response::from_bytes(buffer).unwrap();

        Ok(response.with_headers(headers))
    } else {
        Response::error("Bad Request - Missing Token", 400)
    }
}

pub async fn handle_root(_req: Request, _ctx: RouteContext<()>) -> worker::Result<Response> {
    Response::from_html(r#"<a href="/api/v1/chart/SOL_USDT/1h">GET</a>"#)
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    router
        .get_async("/", handle_root)
        .get_async("/api/v1/chart/:token/:timeframe", handle_chart)
        .run(req, env)
        .await
}
