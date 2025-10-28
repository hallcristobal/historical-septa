use actix_web::{
    HttpRequest, HttpResponse, Responder, ResponseError,
    error::QueryPayloadError,
    http::StatusCode,
    web::{self, Json, QueryConfig},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::Arc};

use crate::{
    SharedAppState,
    db::{QueryOrdering, tracking::Changed},
    septa::{
        query_builder::QueryBuilder,
        train_view::{TrainView, enforce_limit_bounds},
    },
};

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.app_data(QueryConfig::default().error_handler(query_error_handler))
        .service(
            web::scope("/api")
                .route("/current", web::get().to(current_trains))
                .route("/train/{id}", web::get().to(get_train))
                .route("/recent_changes", web::get().to(most_recent_changes))
                .route("/query", web::post().to(query_train)),
        );
}

#[derive(Debug, Serialize)]
struct QeResponse {
    source: String,
    error: String,
}
impl Display for QeResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Query Error: {}", self.error)
    }
}
impl ResponseError for QeResponse {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::BadRequest().json(self)
    }
}

fn query_error_handler(err: QueryPayloadError, req: &HttpRequest) -> actix_web::Error {
    let QueryPayloadError::Deserialize(err) = err else {
        return err.into();
    };

    QeResponse {
        source: req.query_string().to_owned(),
        error: err.to_string(),
    }
    .into()
}

#[derive(Deserialize, Debug)]
pub struct GetCurrentQuery {
    all: Option<bool>,
    line: Option<String>,
    limit: Option<i64>,
}
async fn current_trains(
    query: web::Query<GetCurrentQuery>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    let two_am_today = chrono::Local::now()
        .with_time(chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap())
        .unwrap()
        .to_utc();
    let count = enforce_limit_bounds(query.limit);
    let all = query.all.unwrap_or(false);
    let line = query.line.as_ref();
    let recent = data
        .read()
        .await
        .train_statuses
        .iter()
        .filter_map(|tv| {
            if let Some(ref mri) = tv.1.most_recent_item {
                if let Some(line) = line {
                    if *line != mri.line {
                        return None;
                    }
                }

                if all {
                    Some(mri.clone())
                } else {
                    if mri.timestamp > two_am_today {
                        Some(mri.clone())
                    } else {
                        None
                    }
                }
            } else {
                None
            }
        })
        .take(count as usize)
        .collect::<Vec<Arc<TrainView>>>();
    #[derive(Serialize)]
    struct Response {
        count: u32,
        statuses: Vec<Arc<TrainView>>,
    }
    Json(Response {
        count: recent.len() as u32,
        statuses: recent,
    })
}

async fn most_recent_changes(data: web::Data<SharedAppState>) -> impl Responder {
    #[derive(Serialize)]
    struct Change {
        trainno: String,
        changes: Vec<Changed>,
    }
    #[derive(Serialize)]
    struct Response {
        statuses: Vec<Change>,
    }

    let until = Utc::now() - chrono::Duration::seconds(10);

    let recent = data
        .read()
        .await
        .train_statuses
        .iter()
        .filter_map(|tv| match tv.1.latest_changes {
            Some(ref changes) => {
                let relevant_changes: Vec<Changed> = changes
                    .iter()
                    .filter(|c| c.changed_at >= until)
                    .map(|c| c.clone())
                    .collect();
                if relevant_changes.len() > 0 {
                    Some(Change {
                        trainno: tv.0.clone(),
                        changes: relevant_changes.clone(),
                    })
                } else {
                    None
                }
            }
            None => None,
        })
        .collect();
    Json(Response { statuses: recent })
}

#[derive(Deserialize)]
struct GetTrainPath {
    id: String,
}
#[derive(Deserialize)]
struct GetTrainQuery {
    limit: Option<i64>,
    before: Option<i64>,
    after: Option<i64>,
    order: Option<QueryOrdering>,
}
async fn get_train(
    path: web::Path<GetTrainPath>,
    query: web::Query<GetTrainQuery>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    #[derive(Serialize)]
    struct Response {
        count: usize,
        records: Vec<TrainView>,
    }
    let pg_pool = data.read().await.pg_pool.clone();

    match TrainView::fetch_for_train(
        pg_pool,
        &path.id,
        query.limit,
        query.before.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        query.after.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        query.order,
    )
    .await
    {
        Ok(records) => Json(Response {
            count: records.len(),
            records,
        }),
        Err(e) => {
            error!("Error fetching: {e}");
            Json(Response {
                count: 0,
                records: Vec::new(),
            })
        }
    }
}

#[derive(Deserialize)]
struct QueryTrainQuery {
    limit: Option<i64>,
    before: Option<i64>,
    after: Option<i64>,
    order: Option<QueryOrdering>,
}
async fn query_train(
    body: web::Json<QueryBuilder>,
    query: web::Query<QueryTrainQuery>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    #[derive(Serialize)]
    struct Response {
        count: usize,
        records: Vec<TrainView>,
    }
    let pg_pool = data.read().await.pg_pool.clone();
    match TrainView::query_trains(
        pg_pool,
        body.0,
        query.limit,
        query.before.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        query.after.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        query.order,
    )
    .await
    {
        Ok(records) => Json(Response {
            count: records.len(),
            records,
        }),
        Err(e) => {
            error!("Error fetching: {e}");
            Json(Response {
                count: 0,
                records: Vec::new(),
            })
        }
    }
}
