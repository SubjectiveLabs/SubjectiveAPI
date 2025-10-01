use http::header::AUTHORIZATION;
use reqwest::RequestBuilder;
use serde::{Deserialize, Deserializer};
use worker::Env;

pub fn add_auth_header(env: &Env) -> Option<impl FnOnce(RequestBuilder) -> RequestBuilder> {
    env.secret("key").ok().map(|key| {
        move |builder: RequestBuilder| {
            builder.header(AUTHORIZATION, format!("apikey {key}").as_str())
        }
    })
}

pub struct TimesResult {
    pub times: Vec<String>,
}

impl<'de> Deserialize<'de> for TimesResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Departures {
            stop_events: Vec<StopEvent>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct StopEvent {
            departure_time_planned: String,
        }

        let departures = Departures::deserialize(deserializer)?;
        let times = departures
            .stop_events
            .into_iter()
            .map(|stop_event| stop_event.departure_time_planned)
            .collect();
        Ok(Self { times })
    }
}

pub mod realtime {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}
