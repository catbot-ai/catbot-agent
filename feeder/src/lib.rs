mod charts;

use charts::candle::Chart;
use chrono_tz::Asia::Tokyo;
use common::binance::fetch_orderbook_depth_usdt;
use common::sources::binance::fetch_binance_kline_usdt;
use common::Kline;
use common::RefinedGraphPredictionResponse;
#[cfg(feature = "service_binding")]
use common::ServiceBinding; // Conditionally import ServiceBinding

use std::ops::Deref;
use worker::*;

// TODO: call service binding
async fn gen_candle(pair_symbol: String, interval: String) -> anyhow::Result<Vec<Kline>> {
    let kline_data_1m = fetch_binance_kline_usdt::<Kline>(&pair_symbol, &interval, 240).await?;
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
    handle_chart_prediction(req, ctx, false).await
}

pub async fn handle_chart_signals(req: Request, ctx: RouteContext<()>) -> worker::Result<Response> {
    handle_chart_prediction(req, ctx, true).await
}

pub async fn handle_chart_prediction(
    req: Request,
    ctx: RouteContext<()>,
    is_signals: bool,
) -> worker::Result<Response> {
    if let Some(pair_symbol) = ctx.param("pair_symbol") {
        // Get fetcher
        let api_url = ctx
            .env
            .secret("PREDICTION_API_URL")
            .expect("Expect PREDICTION_API_URL")
            .to_string();

        // Get interval
        let binding = "1h".to_string();
        let interval = ctx.param("interval").unwrap_or(&binding);

        // Finalize api_url
        let relative_path = format!("{pair_symbol}/{interval}");
        let api_url_string = format!("{api_url}/{relative_path}");
        let api_url = Url::parse(&api_url_string).unwrap();

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
            interval.deref().to_string(),
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
        //     title: format!("{pair_symbol} {interval}"),
        // };

        let orderbook = fetch_orderbook_depth_usdt(&pair_symbol, 2000).await;
        let orderbook = match orderbook {
            Ok(orderbook) => orderbook,
            Err(error) => {
                return Response::error(format!("Bad Request - Missing orderbook: {error}"), 400)
            }
        };

        let signals = if is_signals {
            // Fetch prediction based on feature flag
            let prediction_result = {
                #[cfg(not(feature = "service_binding"))]
                {
                    fetch_graph_prediction(api_url, &pair_symbol, interval, None)
                        .await
                        .map_err(|e| {
                            worker::Error::RustError(format!(
                                "Failed direct prediction fetch: {}",
                                e
                            ))
                        })
                }

                #[cfg(feature = "service_binding")]
                {
                    // Use the ServiceBinding helper
                    let fetcher = ctx.env.service("COOKER")?;
                    ServiceBinding::new(fetcher)
                        .with_request(req)
                        .fetch::<RefinedGraphPredictionResponse>(api_url.path())
                        .await
                        .map_err(|e| {
                            worker::Error::RustError(format!(
                                "Failed service binding prediction fetch: {}",
                                e
                            ))
                        })
                }
            };

            let predicted = match prediction_result {
                Ok(predicted_candle_data) => predicted_candle_data,
                Err(worker_err) => {
                    // Log the underlying error if possible
                    console_error!("Prediction fetch failed: {}", worker_err);
                    // Return a worker::Response error
                    return Response::error(
                        format!("Bad Request - Missing prediction data: {}", worker_err),
                        400,
                    );
                }
            };
            predicted.signals
        } else {
            vec![]
        };

        // Get image
        let buffer_result = Chart::new(interval, Tokyo)
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
            .with_signals(signals)
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
    Response::from_html("<a href=\"/api/v1/chart/SOL_USDT/1h\">/api/v1/chart/SOL_USDT/1h</a><hr>")
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    router
        .get_async("/", handle_root)
        // .get_async("/api/v1/suggest/:pair_symbol/:interval", handle_hello)
        .get_async("/api/v1/chart/:pair_symbol/:interval", handle_chart)
        .get_async(
            "/api/v1/chart_signals/:pair_symbol/:interval",
            handle_chart_signals,
        )
        .run(req, env)
        .await
}
