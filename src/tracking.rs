use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Float(f32),
    Int(i32),
}

#[derive(Debug, Serialize, Clone)]
pub struct Changed {
    #[serde(serialize_with = "crate::serde_utils::serialize_date_time")]
    pub timestamp: DateTime<Utc>,
    pub key: String,
    pub old_value: Value,
    pub new_value: Value,
}

pub struct Tracking<T> {
    pub most_recent_timestamp: DateTime<Utc>,
    pub most_recent_item: Option<Arc<T>>,
    pub items: Vec<Arc<T>>,
    pub latest_changes: Option<Vec<Changed>>,
}

impl<T> Default for Tracking<T> {
    fn default() -> Self {
        Tracking {
            most_recent_timestamp: Utc.timestamp_opt(0, 0).unwrap(),
            most_recent_item: None,
            items: Vec::new(),
            latest_changes: None,
        }
    }
}
// #[derive(Serialize)]
// struct Response<'a> {
//     trainno: &'a str,
//     #[serde(
//         serialize_with = "crate::serde_utils::serialize_date_time"
//     )]
//     timestamp: &'a DateTime<Local>,
//     changes: &'a Vec<Changed>,
// }
// let changes = serde_json::to_string(&Response {
//     trainno: &most_recent.trainno,
//     timestamp: &timestamp,
//     changes: &changes,
// })
// .unwrap();
// println!("{changes}");
