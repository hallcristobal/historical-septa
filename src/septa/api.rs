use chrono::{DateTime, Utc};
use reqwest::{self, Result};

use crate::septa::train_view;
pub async fn fetch_train_view() -> Result<(DateTime<Utc>, train_view::Content)> {
    let url = "https://www3.septa.org/api/TrainView/index.php";
    let response = reqwest::get(url).await?;

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

    let body = response.json::<train_view::Content>().await?;

    Ok((date, body))
}
