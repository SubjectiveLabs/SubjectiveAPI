use std::collections::HashMap;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use prost::Message;
use reqwest::Client;
use serde::Serialize;
use serde_json::to_string;
use tap::Pipe;
use worker::{Request, Response, RouteContext};

use crate::common::{add_auth_header, realtime::FeedMessage};

#[derive(Serialize)]
struct TimeResult {
    arrival: DateTime<Utc>,
    delay_sec: i32,
}

#[derive(Serialize)]
struct TimesResult {
    times: Vec<TimeResult>,
    updated_at: Option<DateTime<Utc>>,
}

pub async fn times(request: Request, context: RouteContext<()>) -> worker::Result<Response> {
    let url = request.url()?;
    let pairs: HashMap<_, _> = url
        .query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let Some(stop_id) = pairs.get("stop_id") else {
        return Response::error("Missing `stop_id` parameter.", 400);
    };
    let Some(route_id) = pairs.get("route_id") else {
        return Response::error("Missing `route_id` parameter.", 400);
    };
    let Some(add_auth_header) = add_auth_header(&context.env) else {
        return Response::error("Missing API key.", 500);
    };
    let response = match Client::new()
        .get("https://api.transport.nsw.gov.au/v1/gtfs/realtime/buses")
        .pipe(add_auth_header)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return Response::error(
                format!(
                    "Error while sending a request to the Transport Open Data 'Public Transport - Realtime Trip Updates API' API:\n\n{error:#?}"
                ),
                500,
            );
        }
    };
    let bytes = match response.bytes().await {
        Ok(text) => text,
        Err(error) => {
            return Response::error(
                format!(
                    "Error while reading bytes from the response from the Transport Open Data 'Public Transport - Realtime Trip Updates API' API:\n\n{error:#?}"
                ),
                500,
            );
        }
    };
    let Ok(message) = FeedMessage::decode(bytes) else {
        return Response::error(
            "Error while decoding the response from the Transport Open Data 'Public Transport - Realtime Trip Updates API' API.",
            500,
        );
    };
    let mut latest = None;
    let times = message
        .entity
        .iter()
        .filter_map(|entity| {
            entity.trip_update.as_ref().map(|trip_update| {
                #[allow(clippy::cast_possible_wrap)]
                if let Some(updated) = trip_update
                    .timestamp
                    .map(|timestamp| timestamp as i64)
                    .and_then(DateTime::from_timestamp_secs)
                    && latest.is_none_or(|earliest| updated > earliest)
                {
                    latest = Some(updated);
                }
                trip_update.stop_time_update.iter().filter_map(|update| {
                    update
                        .arrival
                        .and_then(|arrival| {
                            arrival
                                .time
                                .map(DateTime::from_timestamp_secs)
                                .map(Option::unwrap)
                                .zip(arrival.delay)
                        })
                        .filter(|_| {
                            trip_update
                                .trip
                                .route_id
                                .as_ref()
                                .is_some_and(|id| *id == *route_id)
                                && update.stop_id.as_ref().is_some_and(|id| *id == *stop_id)
                        })
                        .map(|(arrival, delay_sec)| TimeResult { arrival, delay_sec })
                })
            })
        })
        .flatten()
        .collect_vec();
    match to_string(&TimesResult {
        times,
        updated_at: latest,
    }) {
        Ok(json) => Response::ok(json),
        Err(error) => Response::error(
            format!("Error while serializing a response: {error:#?}"),
            500,
        ),
    }
}
