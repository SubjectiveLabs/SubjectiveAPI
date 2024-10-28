#![allow(clippy::future_not_send)]
#![warn(clippy::unwrap_used)]

use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use console_error_panic_hook::set_once;
use csv::Reader;
use http::header::AUTHORIZATION;
use itertools::Itertools;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use tap::Pipe;
use worker::{event, Context, Env, Request, Response, RouteContext, Router};
use zip::ZipArchive;

#[event(fetch)]
async fn fetch(request: Request, env: Env, _context: Context) -> worker::Result<Response> {
    set_once();
    let router = Router::new()
        .get_async("/routes", routes)
        .get_async("/stops", stops);
    router.run(request, env).await
}

async fn routes(request: Request, context: RouteContext<()>) -> worker::Result<Response> {
    #[derive(Deserialize)]
    struct RoutesResult {
        #[serde(rename = "ROUTE")]
        routes: Vec<Route>,
    }

    #[derive(Deserialize, Serialize)]
    struct Route {
        #[serde(rename(deserialize = "service_direction_name"))]
        full_name: String,
        #[serde(rename(deserialize = "contract_id"))]
        agency: String,
        #[serde(rename(deserialize = "efa_route_name"))]
        name: String,
        #[serde(rename(deserialize = "gtfs_route_id_out"))]
        id: String,
    }
    let url = request.url()?;
    let mut pairs = url.query_pairs();
    let Some((key, value)) = pairs.next() else {
        return Response::error("Missing `query` parameter.", 400);
    };
    if pairs.next().is_some() {
        return Response::error("More than one query parameter was provided.", 400);
    }
    if key != "query" {
        return Response::error("The query parameter must be named `query`.", 400);
    }
    let route = value.to_string();
    let Some(add_auth_header) = add_auth_header(&context.env) else {
        return Response::error("Missing API key.", 500);
    };
    let response = match Client::new()
        .get("https://api.transport.nsw.gov.au/v1/routes")
        .query(&[("route", route)])
        .pipe(add_auth_header)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return Response::error(
                format!(
                "Error while sending a request to the Transport Open Data 'Transport Routes' API:\n\n{error:#?}"
            ),
                500,
            );
        }
    };
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return Response::error(format!("Error while reading text from the response from the Transport Open Data 'Transport Routes' API:\n\n{error:#?}"), 500);
        }
    };
    let result = match from_str::<RoutesResult>(&text) {
        Ok(result) => result,
        Err(error) => {
            return Response::error(format!("Error while parsing JSON from the response from the Transport Open Data 'Transport Routes' API:\n\n{error:#?}"), 500);
        }
    };
    Response::from_json(&result.routes)
}

fn add_auth_header(env: &Env) -> Option<impl FnOnce(RequestBuilder) -> RequestBuilder> {
    env.secret("key").ok().map(|key| {
        move |builder: RequestBuilder| {
            builder.header(AUTHORIZATION, format!("apikey {key}").as_str())
        }
    })
}

#[allow(clippy::too_many_lines)]
async fn stops(request: Request, context: RouteContext<()>) -> worker::Result<Response> {
    #[derive(Deserialize, Serialize, Debug)]
    struct Stop {
        #[serde(rename(deserialize = "stop_name"))]
        name: String,
        #[serde(rename(deserialize = "stop_lat"))]
        latitude: f64,
        #[serde(rename(deserialize = "stop_lon"))]
        longitude: f64,
    }
    let url = request.url()?;
    let pairs: HashMap<_, _> = url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let Some(add_auth_header) = add_auth_header(&context.env) else {
        return Response::error("Missing API key.", 500);
    };
    let response = match Client::new()
        .get(format!(
            "https://api.transport.nsw.gov.au/v1/gtfs/schedule/buses/{}",
            pairs["agency"]
        ))
        .pipe(add_auth_header)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return Response::error(format!("Error while sending a request to the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500);
        }
    };
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => {
            return Response::error(format!("Error while reading bytes from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500);
        }
    };
    let mut archive = match ZipArchive::new(Cursor::new(bytes)) {
        Ok(archive) => archive,
        Err(error) => {
            return Response::error(format!("Error while reading the ZIP archive from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500);
        }
    };
    let mut read = |name: &str| {
        let mut string = String::new();
        let mut file = match archive.by_name(name) {
            Ok(file) => file,
            Err(error) => {
                return Err(Response::error(format!("Error while reading the '{name}' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500));
            }
        };
        if let Err(error) = file.read_to_string(&mut string) {
            return Err(Response::error(format!("Error while reading the '{name}' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500));
        };
        Ok(Reader::from_reader(Cursor::new(string)))
    };
    let trips = match read("trips.txt") {
        Ok(trips) => trips,
        Err(response) => return response,
    };
    let stop_times = match match read("stop_times.txt") {
        Ok(stop_times) => stop_times,
        Err(response) => return response,
    }
    .into_records()
    .try_collect::<_, Vec<_>, _>()
    {
        Ok(stop_times) => stop_times,
        Err(error) => {
            return Response::error(format!("Error while reading the stop times from the 'stop_times.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500);
        }
    };
    let mut stops = match read("stops.txt") {
        Ok(stops) => stops,
        Err(response) => return response,
    };
    let trip_ids = match trips
        .into_records()
        .filter_map(|record| {
            record
                .map(|record| (record[0] == pairs["id"]).then(|| record[2].to_string()))
                .transpose()
        })
        .try_collect::<_, Vec<_>, _>()
    {
        Ok(trip_ids) => trip_ids,
        Err(error) => {
            return Response::error(format!("Error while reading the trip IDs from the 'trips.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500);
        }
    };
    let stop_ids = trip_ids.into_iter().flat_map(|trip_id| {
        stop_times
            .iter()
            .filter(move |record| (record[0] == trip_id))
            .map(|record| record[3].to_string())
    });
    let records = stops.records().collect_vec();
    let headers = match stops.headers() {
        Ok(headers) => headers,
        Err(error) => {
            return Response::error(format!("Error while reading the headers from the 'stops.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500);
        }
    };
    {
        enum StopError<'a> {
            NotExactlyOne(String),
            DeserializeError(csv::Error),
            RecordReadError(&'a csv::Error),
        }
        let stops = match stop_ids
            .into_iter()
            .unique()
            .collect_vec()
            .pipe(|stop_ids| {
                stop_ids
                    .into_iter()
                    .map(|stop_id| {
                        records
                            .iter()
                            .filter_map(|record| {
                                record
                                    .as_ref()
                                    .map(|record| {
                                        (record[0] == stop_id)
                                            .then(|| record.deserialize::<Stop>(Some(headers)))
                                    })
                                    .transpose()
                            })
                            .collect_vec()
                            .into_iter()
                            .exactly_one()
                            .map_err(|_| stop_id)
                            .pipe(|result| match result {
                                Ok(Ok(Ok(stop))) => Ok(stop),
                                Ok(Ok(Err(error))) => Err(StopError::DeserializeError(error)),
                                Ok(Err(error)) => Err(StopError::RecordReadError(error)),
                                Err(error) => Err(StopError::NotExactlyOne(error)),
                            })
                    })
                    .try_collect::<_, Vec<_>, _>()
            }) {
            Ok(stops) => stops,
            Err(StopError::NotExactlyOne(stop_id)) => {
                return Response::error(format!("Error while reading the stops from the 'stops.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API.\n\nThere was not exactly one stop which matched the stop ID `{stop_id}`."), 500);
            }
            Err(
                StopError::DeserializeError(ref error) | StopError::RecordReadError(&ref error),
            ) => {
                return Response::error(format!("Error while reading the stops from the 'stops.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API.\n\n{error:#?}"), 500);
            }
        };
        Response::from_json(&stops)
    }
}
