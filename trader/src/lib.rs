use worker::*;

async fn get_signals(timeframes: Vec<&str>) -> Result<Response> {
    // This shoud start the action
    // 1. get signals from worker KV by interval:time_stamp `1h:`

    // TODO
    let html = format!("Fetching signals for timeframes: {:?}", timeframes);
    Response::from_html(html)
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let router = Router::new();

    router
        .get_async("/perps", |_req, _ctx| async move {
            let timeframes = vec!["15m", "1h", "4h", "1d"];
            get_signals(timeframes).await
        })
        .run(req, env)
        .await
}
