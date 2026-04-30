use actix_web::{
    HttpRequest, HttpResponse, Responder, ResponseError,
    error::QueryPayloadError,
    http::StatusCode,
    web::{self, Json, QueryConfig},
};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::Arc};

use septa::db::{QueryOrdering, tracking::Changed};

use septa::septa::{
    query_builder::QueryBuilder,
    train_view::{TrainView, enforce_limit_bounds},
};

use crate::SharedAppState;

use actix_web::{App, HttpServer};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use septa::{db::tracking::Tracking, septa::train_view::TrainView};

struct AppState {
    train_statuses: HashMap<String, Tracking<TrainView>>,
    pg_pool: PgPool,
}
type SharedAppState = Arc<RwLock<AppState>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().unwrap();
    pretty_env_logger::init_timed();

    let state = AppState {
        train_statuses: HashMap::new(),
        pg_pool: septa::db::init().await.unwrap(),
    };
    let state = Arc::new(RwLock::new(state));
    let backfilled = populate_known_statuses(state.clone()).await?;
    info!("Backfilled {} statuses during startup.", backfilled);

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(state.clone()))
            .configure(web::routes)
    })
    .bind(("0.0.0.0", 8081))
    .unwrap()
    .run()
    .await
    .map_err(|e| e.into())
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
                if let Some(line) = line
                    && *line != mri.line
                {
                    return None;
                }

                if all || mri.timestamp > two_am_today {
                    Some(mri.clone())
                } else {
                    None
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
    (
        Json(Response {
            count: recent.len() as u32,
            statuses: recent,
        }),
        StatusCode::OK,
    )
}
