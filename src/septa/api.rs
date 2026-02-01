use std::fmt::Debug;

use chrono::{DateTime, Utc};
use reqwest;
use uuid::Uuid;

use crate::{
    db::tracking::FailedFetchError, septa::{content::Content, train_view::{SeptaTrainView, TrainView}},
};
fn err_to_string<E: Debug>(e: E) -> String {
    format!("{:?}", e)
}

pub async fn fetch_train_view() -> anyhow::Result<Content, FailedFetchError> {
    let url = "https://www3.septa.org/api/TrainView/index.php";
    let response = reqwest::get(url)
        .await
        .map_err(|e| FailedFetchError(chrono::Utc::now(), format!("{e:?}")))?;

    info!("Fetched with status: {}", response.status());
    trace!("Headers:\n{:#?}", response.headers());
    let date: DateTime<Utc> = response
        .headers()
        .get("date")
        .map(|hv| {
            DateTime::parse_from_rfc2822(hv.to_str().unwrap())
                .unwrap()
                .to_utc()
        })
        .unwrap_or(chrono::Utc::now());
    let file_id = Uuid::new_v4();

    match response
        .text()
        .await
        .map_err(err_to_string)
        .and_then(|body| {
            serde_json::from_str::<Vec<SeptaTrainView>>(&body)
                .map(|v| (body, v))
                .map_err(err_to_string)
        }) {
        Ok((raw, body)) => Ok(Content {
            id: file_id,
            timestamp: date,
            raw,
            trains: body.into_iter().map(|stv| {
                let mut tv: TrainView = stv.into();
                tv.file_id = file_id;
                tv
            }).collect(),
        }),
        Err(e) => Err(FailedFetchError(date, e)),
    }
}
