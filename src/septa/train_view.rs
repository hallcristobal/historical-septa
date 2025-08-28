use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::tracking::{Changed, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainView {
    #[serde(
        skip_deserializing,
        default,
        serialize_with = "crate::serde_utils::serialize_date_time"
    )]
    pub timestamp: chrono::DateTime<Utc>,
    // #[serde(deserialize_with = "crate::serde_utils::deserialize_f32_string")]
    // pub lat: f32,
    // #[serde(deserialize_with = "crate::serde_utils::deserialize_f32_string")]
    // pub lon: f32,
    pub trainno: String,
    pub service: String,
    pub dest: String,
    pub currentstop: String,
    pub nextstop: String,
    pub line: String,
    pub consist: String,
    // #[serde(deserialize_with = "crate::serde_utils::deserialize_opt_f32_string")]
    // pub heading: Option<f32>,
    pub late: i32,
    #[serde(rename = "SOURCE")]
    pub source: String,
    // #[serde(rename = "TRACK")]
    // pub track: String,
    // #[serde(rename = "TRACK_CHANGE")]
    // pub track_change: String,
}

impl TrainView {
    pub fn has_changed(&self, prev: &TrainView) -> Option<Vec<Changed>> {
        let mut changed = vec![];
        if self.trainno != prev.trainno {
            return None;
        }

        if self.service != prev.service {
            changed.push(Changed {
                timestamp: self.timestamp,
                key: String::from("service"),
                old_value: Value::String(prev.service.to_owned()),
                new_value: Value::String(self.service.to_owned()),
            });
        }
        if self.late != prev.late {
            changed.push(Changed {
                timestamp: self.timestamp,
                key: String::from("late"),
                old_value: Value::Int(prev.late),
                new_value: Value::Int(self.late),
            });
        }
        // if self.track != prev.track {
        //     changed.push(Changed {
        //         timestamp: self.timestamp,
        //         key: String::from("track"),
        //         old_value: Value::String(prev.track.to_owned()),
        //         new_value: Value::String(self.track.to_owned()),
        //     });
        // }

        if changed.len() > 0 {
            Some(changed)
        } else {
            None
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct Content(pub Vec<TrainView>);
