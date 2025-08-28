use std::fmt::Debug;

use chrono::{DateTime, Utc};
use reqwest;

use crate::{
    septa::train_view::{Content, TrainView},
    tracking::FailedFetchError,
};
fn err_to_string<E: Debug>(e: E) -> String {
    format!("{:?}", e)
}

pub async fn fetch_train_view() -> anyhow::Result<Content, FailedFetchError> {
    let url = "https://www3.septa.org/api/TrainView/index.php";
    let response = reqwest::get(url)
        .await
        .map_err(|e| FailedFetchError(chrono::Utc::now(), format!("{e:?}")))?;

    println!("Fetched with status: {}", response.status());
    // println!("Headers:\n{:#?}", response.headers());
    let date: DateTime<Utc> = response
        .headers()
        .get("date")
        .map(|hv| {
            DateTime::parse_from_rfc2822(hv.to_str().unwrap())
                .unwrap()
                .to_utc()
        })
        .unwrap_or(chrono::Utc::now());

    // match response.text().await {
    //     Ok(raw) => match serde_json::from_str::<Vec<TrainView>>(&raw) {
    //         Ok(body) => {}
    //     },
    // }
    match response
        .text()
        .await
        .map_err(err_to_string)
        .and_then(|body| {
            serde_json::from_str::<Vec<TrainView>>(&body)
                .map(|v| (body, v))
                .map_err(err_to_string)
        }) {
        Ok((raw, body)) => Ok(Content {
            timestamp: date,
            raw,
            trains: body,
        }),
        Err(e) => Err(FailedFetchError(date, e).into()),
    }
}
