#![allow(clippy::future_not_send)]
#![warn(clippy::unwrap_used)]

use console_error_panic_hook::set_once;
use transport::{routes, stops, times};
use worker::{event, Context, Env, Request, Response, Router};

mod transport;
mod icon;

#[event(fetch)]
async fn fetch(request: Request, env: Env, _context: Context) -> worker::Result<Response> {
    set_once();
    let router = Router::new()
        .get_async("/v1/transport/routes", routes)
        .get_async("/v1/transport/stops", stops)
        .get_async("/v1/transport/times", times)
        .get_async("/v1/icon/choose", icon::choose);
    router.run(request, env).await
}
