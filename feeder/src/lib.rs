mod charts;

use charts::candle::{Chart, ChartMetaData};
use chrono_tz::Asia::Tokyo;
use common::sources::binance::fetch_binance_kline_data;
use common::Kline;
use std::ops::Deref;
use worker::*;

// TODO: call service binding
async fn gen_candle(pair_symbol: String, timeframe: String) -> anyhow::Result<Vec<Kline>> {
    let kline_data_1m = fetch_binance_kline_data::<Kline>(&pair_symbol, &timeframe, 300).await?;
    Ok(kline_data_1m)
}

// TODO: pixel font
const DEFAULT_FONT_NAME: &str = "Roboto-Light.ttf";

pub async fn handle_chart(_: Request, ctx: RouteContext<()>) -> worker::Result<Response> {
    if let Some(pair_symbol) = ctx.param("pair_symbol") {
        // Get timeframe
        let binding = "5m".to_string();
        let timeframe = ctx.param("timeframe").unwrap_or(&binding);

        // Get font
        let kv_store = ctx.kv("ASSETS").unwrap();
        let font_data = kv_store
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

        let candle_data = match candle_data {
            Ok(candle_data) => candle_data,
            Err(error) => {
                return Response::error(format!("Bad Request - Missing Data: {error}"), 400)
            }
        };

        // Get chart metadata
        let chart_metadata = ChartMetaData {
            title: format!("{pair_symbol} {timeframe}"),
        };

        // Get image
        let buffer_result = Chart::new(Tokyo)
            .with_past_candle(candle_data)
            .with_title(&chart_metadata.title)
            .with_font_data(font_data)
            .with_macd()
            .with_bollinger_band()
            .build();

        // Handle
        let buffer = match buffer_result {
            Ok(buffer) => buffer,
            Err(error) => {
                return Response::error(format!("Bad Request - Missing Data: {error}"), 400)
            }
        };

        let mut headers = Headers::new();
        headers.set("content-type", "image/png")?;
        let response = Response::from_bytes(buffer).unwrap();

        Ok(response.with_headers(headers))
    } else {
        Response::error("Bad Request - Missing Token", 400)
    }
}

pub async fn handle_root(_req: Request, _ctx: RouteContext<()>) -> worker::Result<Response> {
    Response::from_html(r#"<a href="/api/v1/chart/SOL_USDT/5m">GET</a>"#)
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    router
        .get_async("/", handle_root)
        .get_async("/api/v1/chart/:pair_symbol/:timeframe", handle_chart)
        .run(req, env)
        .await
}
