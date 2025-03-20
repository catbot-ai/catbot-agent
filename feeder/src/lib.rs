mod charts;

use charts::candle::Chart;
use chrono_tz::Asia::Tokyo;
use common::binance::fetch_orderbook_depth;
use common::cooker::fetch_graph_prediction_from_worker;
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
const DEFAULT_FONT_NAME: &str = "RobotoMono-Regular.ttf";

pub async fn cooker(req: Request, ctx: RouteContext<()>) -> worker::Result<Response> {
    // Fetch the response from the COOKER service
    let fetcher = ctx.env.service("COOKER")?;
    let http_request: worker::HttpRequest = req.try_into()?;
    let resp = fetcher.fetch_request(http_request).await?;

    // Convert to cf response
    let cf_response: Response = resp.try_into()?;

    // Convert the JSON value to a string and return it as a Response
    Response::from_body(cf_response.body().clone())
}

pub async fn handle_chart(req: Request, ctx: RouteContext<()>) -> worker::Result<Response> {
    if let Some(pair_symbol) = ctx.param("pair_symbol") {
        // Get fetcher
        let api_url = ctx
            .env
            .secret("PREDICTION_API_URL")
            .expect("Expect PREDICTION_API_URL")
            .to_string();

        // Get timeframe
        let binding = "1h".to_string();
        let timeframe = ctx.param("timeframe").unwrap_or(&binding);

        // Finalize api_url
        let api_url = format!("{api_url}/{pair_symbol}/{timeframe}");

        // Get font
        let kv_store = ctx.kv("ASSETS").unwrap();
        let font_data = kv_store
            .get(DEFAULT_FONT_NAME)
            .bytes()
            .await
            .unwrap()
            .unwrap();

        // Get data
        let pair_symbol = pair_symbol.clone();
        let candle_data = gen_candle(
            pair_symbol.deref().to_string(),
            timeframe.deref().to_string(),
        )
        .await;

        let candle_data = match candle_data {
            Ok(candle_data) => candle_data,
            Err(error) => {
                return Response::error(format!("Bad Request - Missing candle data: {error}"), 400)
            }
        };

        // TODO: Define chart metadata
        // let chart_metadata = ChartMetaData {
        //     title: format!("{pair_symbol} {timeframe}"),
        // };

        let orderbook = fetch_orderbook_depth(&pair_symbol, 1000).await;
        let orderbook = match orderbook {
            Ok(orderbook) => orderbook,
            Err(error) => {
                return Response::error(format!("Bad Request - Missing orderbook: {error}"), 400)
            }
        };

        // TODO: Extract signal for plot the chart
        #[cfg(not(feature = "service_binding"))]
        let prediction = fetch_graph_prediction(&api_url, &pair_symbol, &timeframe, None).await;

        #[cfg(feature = "service_binding")]
        let fetcher = ctx.env.service("COOKER")?;
        let prediction =
            fetch_graph_prediction_from_worker(req, &fetcher, &pair_symbol, timeframe).await;

        let predicted = match prediction {
            Ok(predicted_candle_data) => predicted_candle_data,
            Err(error) => {
                return Response::error(
                    format!("Bad Request - Missing prediction data from {api_url}: {error}"),
                    400,
                )
            }
        };

        // Get image
        let buffer_result = Chart::new(timeframe, Tokyo)
            .with_past_candle(candle_data)
            // So sad this didn't work as expected due to poor results
            // .with_predicted_candle(predicted_klines)
            .with_title(&pair_symbol)
            .with_font_data(font_data)
            .with_volume()
            .with_macd()
            .with_stoch_rsi()
            .with_orderbook(orderbook)
            .with_bollinger_band()
            // .with_past_signals(predicted.signals)
            .with_signals(predicted.signals)
            .build();

        // Handle
        let buffer = match buffer_result {
            Ok(buffer) => buffer,
            Err(error) => {
                return Response::error(format!("Bad Request - Missing image data: {error}"), 400)
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

pub async fn handle_root(_: Request, _ctx: RouteContext<()>) -> worker::Result<Response> {
    Response::from_html("<a href=\"/api/v1/chart/SOL_USDC/1h\">/api/v1/chart/SOL_USDC/1h</a><hr>")
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    router
        .get_async("/", handle_root)
        // .get_async("/api/v1/suggest/:pair_symbol/:timeframe", handle_hello)
        .get_async("/api/v1/chart/:pair_symbol/:timeframe", handle_chart)
        .run(req, env)
        .await
}
