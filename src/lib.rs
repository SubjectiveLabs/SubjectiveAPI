#![allow(clippy::future_not_send)]
#![warn(clippy::unwrap_used)]

use console_error_panic_hook::set_once;
use transport_v1::{routes, stops, times as times_v1};
use transport_v2::times as times_v2;
use worker::{event, Context, Env, Request, Response, Router};

mod icon;
mod transport_v1;
mod transport_v2;

mod common;

#[event(fetch)]
async fn fetch(request: Request, env: Env, _context: Context) -> worker::Result<Response> {
    set_once();
    let router = Router::new()
        .get_async("/v1/transport/routes", routes)
        .get_async("/v1/transport/stops", stops)
        .get_async("/v1/transport/times", times_v1)
        .get_async("/v2/transport/times", times_v2)
        .get_async("/v1/icon/choose", icon::choose);
    router.run(request, env).await
}
