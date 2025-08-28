use std::{fmt::Display, sync::Arc};

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use sqlx::{PgPool, prelude::FromRow};
use uuid::Uuid;

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Float(f32),
    Int(i32),
}

impl Value {
    pub fn string_name(&self) -> String {
        match self {
            Value::String(_) => "String".into(),
            Value::Float(_) => "Float".into(),
            Value::Int(_) => "Integer".into(),
        }
    }

    pub fn to_sql_fields(&self) -> (String, String) {
        match self {
            Value::String(val) => ("String".into(), val.clone()),
            Value::Float(val) => ("Float".into(), val.to_string()),
            Value::Int(val) => ("Integer".into(), val.to_string()),
        }
    }
}

#[derive(Debug, Serialize, Clone, FromRow)]
pub struct Changed {
    pub id: Uuid,
    pub trainno: String,
    pub record_id: Uuid,
    #[serde(serialize_with = "crate::serde_utils::serialize_date_time")]
    pub changed_at: DateTime<Utc>,
    pub field: String,
    pub old_value: Value,
    pub new_value: Value,
    #[sqlx(rename = "type")]
    pub _type: String,
}

pub struct Tracking<T> {
    pub most_recent_timestamp: DateTime<Utc>,
    pub most_recent_item: Option<Arc<T>>,
    // pub items: Vec<Arc<T>>,
    pub latest_changes: Option<Vec<Changed>>,
}

impl<T> Default for Tracking<T> {
    fn default() -> Self {
        Tracking {
            most_recent_timestamp: Utc.timestamp_opt(0, 0).unwrap(),
            most_recent_item: None,
            // items: Vec::new(),
            latest_changes: None,
        }
    }
}

#[derive(Serialize)]
pub struct Fetch {
    pub id: Uuid,
    #[serde(serialize_with = "crate::serde_utils::serialize_date_time")]
    pub timestamp: DateTime<Utc>,
    pub status: String,
    pub result: Option<String>,
}

impl Fetch {
    pub fn new(timestamp: DateTime<Utc>, status: String, result: Option<String>) -> Self {
        Fetch {
            id: Uuid::new_v4(),
            timestamp,
            status,
            result,
        }
    }

    pub async fn store_fetch(&self, pg_pool: PgPool) -> anyhow::Result<()> {
        sqlx::query!(
            "INSERT INTO fetches (id, timestamp, status, result) VALUES ($1, $2, $3, $4)",
            self.id,
            self.timestamp.naive_utc(),
            self.status,
            self.result
        )
        .execute(&pg_pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct FailedFetchError(pub DateTime<Utc>, pub String);
impl Display for FailedFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for FailedFetchError {}
