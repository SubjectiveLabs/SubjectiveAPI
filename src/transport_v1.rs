use std::{
    collections::HashMap,
    fmt::{self, Debug, Formatter},
    io::{Cursor, Read},
};

use csv::Reader;
use itertools::Itertools;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use tap::Pipe;
use worker::{Request, Response, RouteContext};
use zip::ZipArchive;

use crate::common::{add_auth_header, TimesResult};

pub async fn routes(request: Request, context: RouteContext<()>) -> worker::Result<Response> {
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

#[allow(clippy::too_many_lines)]
pub async fn stops(request: Request, context: RouteContext<()>) -> worker::Result<Response> {
    let url = request.url()?;
    let pairs: HashMap<_, _> = url
        .query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
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
        let mut file = match archive.by_name(name) {
            Ok(file) => file,
            Err(error) => {
                return Err(Response::error(format!("Error while reading the '{name}' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500));
            }
        };
        let mut buffer = Vec::new();
        if let Err(error) = file.read_to_end(&mut buffer) {
            return Err(Response::error(format!("Error while reading the '{name}' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500));
        }
        Ok(Reader::from_reader(Cursor::new(buffer)))
    };
    let trips = match read("trips.txt") {
        Ok(trips) => trips,
        Err(response) => return response,
    };
    let stop_times = match read("stop_times.txt") {
        Ok(stop_times) => stop_times,
        Err(response) => return response,
    }
    .into_records();
    let mut stops = match read("stops.txt") {
        Ok(stops) => stops,
        Err(response) => return response,
    };
    let trip_id = match match trips.into_records().find_map(|record| {
            record
                .map(|record| (record[0] == pairs["id"]).then(|| record[2].to_string()))
                .transpose()
        }) {
        Some(result) => {result},
        None => {return Response::error("No trip IDs match the given route in the 'trips.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API.".to_string(), 500)},
    } {
        Ok(trip_id) => trip_id,
        Err(error) => {
            return Response::error(format!("Error while reading the trip IDs from the 'trips.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500);
        }
    };
    let stop_ids = stop_times
        .filter_ok(move |record| record[0] == *trip_id)
        .map_ok(|record| record[3].to_string());
    let records = stops.records().collect_vec();
    let headers = match stops.headers() {
        Ok(headers) => headers,
        Err(error) => {
            return Response::error(format!("Error while reading the headers from the 'stops.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API:\n\n{error:#?}"), 500);
        }
    };
    {
        enum MaybeBorrowed<'a, T> {
            Borrowed(&'a T),
            Owned(T),
        }
        impl<'a, T> From<&'a T> for MaybeBorrowed<'a, T> {
            fn from(value: &'a T) -> Self {
                MaybeBorrowed::Borrowed(value)
            }
        }
        impl<T> From<T> for MaybeBorrowed<'_, T> {
            fn from(value: T) -> Self {
                MaybeBorrowed::Owned(value)
            }
        }
        impl<T: Debug> Debug for MaybeBorrowed<'_, T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                match self {
                    Self::Borrowed(inner) => inner.fmt(f),
                    Self::Owned(inner) => inner.fmt(f),
                }
            }
        }
        enum StopError<'a> {
            NotExactlyOne(String),
            DeserializeError(csv::Error),
            RecordReadError(&'static str, MaybeBorrowed<'a, csv::Error>),
        }
        #[derive(Deserialize, Serialize, Debug)]
        struct Stop {
            #[serde(rename(deserialize = "stop_id"))]
            id: String,
            #[serde(rename(deserialize = "stop_name"))]
            name: String,
            #[serde(rename(deserialize = "stop_lat"))]
            latitude: f64,
            #[serde(rename(deserialize = "stop_lon"))]
            longitude: f64,
        }
        let stops = match stop_ids
            .map(|stop_id| {
                let stop_id = match stop_id {
                    Ok(stop_id) => stop_id,
                    Err(error) => {
                        return Err(StopError::RecordReadError("stop_times", error.into()))
                    }
                };
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
                        Ok(Err(error)) => Err(StopError::RecordReadError("stops", error.into())),
                        Err(error) => Err(StopError::NotExactlyOne(error)),
                    })
            })
            .try_collect::<_, Vec<_>, _>()
        {
            Ok(stops) => stops,
            Err(StopError::NotExactlyOne(stop_id)) => {
                return Response::error(format!("Error while reading the stops from the 'stops.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API.\n\nThere was not exactly one stop which matched the stop ID `{stop_id}`."), 500);
            }
            Err(StopError::DeserializeError(error)) => {
                return Response::error(format!("Error while reading the stops from the 'stops.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API.\n\n{error:#?}"), 500);
            }
            Err(StopError::RecordReadError(file, error)) => {
                return Response::error(format!("Error while reading the stops from the '{file}.txt' file from the Transport Open Data 'Public Transport - Timetables - For Realtime' API.\n\n{error:#?}"), 500);
            }
        };
        Response::from_json(&stops)
    }
}

pub async fn times(request: Request, context: RouteContext<()>) -> worker::Result<Response> {
    let url = request.url()?;
    let pairs: HashMap<_, _> = url
        .query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let Some(id) = pairs.get("id") else {
        return Response::error("Missing `id` parameter.", 400);
    };
    let Some(add_auth_header) = add_auth_header(&context.env) else {
        return Response::error("Missing API key.", 500);
    };
    let response = match Client::new()
        .get(format!(
            "https://api.transport.nsw.gov.au/v1/tp/departure_mon?outputFormat=rapidJSON&coordOutputFormat=EPSG%3A4326&mode=direct&type_dm=stop&name_dm={id}&departureMonitorMacro=true&excludedMeans=checkbox&exclMOT_1=1&exclMOT_2=1&exclMOT_4=1&exclMOT_7=1&exclMOT_9=1&TfNSWDM=true&version=10.2.1.42"
        ))
        .pipe(add_auth_header)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return Response::error(format!("Error while sending a request to the Transport Open Data 'Trip Planner APIs' API:\n\n{error:#?}"), 500);
        }
    };
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return Response::error(format!("Error while reading text from the response from the Transport Open Data 'Trip Planner APIs' API:\n\n{error:#?}"), 500);
        }
    };
    let result: TimesResult = match from_str(&text) {
        Ok(result) => result,
        Err(error) => {
            return Response::error(format!("Error while parsing JSON from the response from the Transport Open Data 'Trip Planner APIs' API:\n\n{error:#?}"), 500);
        }
    };
    Response::from_json(&result.times)
}
