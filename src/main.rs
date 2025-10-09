#[macro_use]
extern crate log;

use actix_web::{App, HttpServer};
use sqlx::PgPool;
use std::{collections::HashMap, env, sync::Arc};
use tokio::sync::RwLock;

use crate::{db::tracking::Tracking, septa::train_view::TrainView};

mod db;
mod septa;
mod serde_utils;
mod web;

struct AppState {
    train_statuses: HashMap<String, Tracking<TrainView>>,
    pg_pool: PgPool,
}
type SharedAppState = Arc<RwLock<AppState>>;

async fn populate_known_statuses(state: SharedAppState) -> anyhow::Result<usize> {
    let train_views = TrainView::get_most_recent_all(state.read().await.pg_pool.clone()).await?;
    let train_statuses = &mut state.write().await.train_statuses;
    train_views.iter().for_each(|train_view| {
        train_statuses.insert(
            train_view.trainno.to_owned(),
            Tracking {
                most_recent_item: Some(Arc::new(train_view.clone())),
                most_recent_timestamp: train_view.timestamp,
                latest_changes: None,
            },
        );
    });
    Ok(train_views.len())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().unwrap();
    pretty_env_logger::init_timed();
    let fetch_and_process_septa = !env::var("NO_FETCH").map(|v| v == "true").unwrap_or(false);

    let state = AppState {
        train_statuses: HashMap::new(),
        pg_pool: db::init().await.unwrap(),
    };
    let state = Arc::new(RwLock::new(state));
    let backfilled = populate_known_statuses(state.clone()).await?;
    info!("Backfilled {} statuses during startup.", backfilled);

    if fetch_and_process_septa {
        info!("Starting Septa processes");
        match septa::processing::start(state.clone()).await {
            Ok((poll_handle, process_handle)) => {
                debug!("Started threads: {:?} {:?}", poll_handle, process_handle);
            }
            Err(e) => {
                error!("Error starting septa threads: {e:?}");
                return Err(e);
            }
        }
    }

    let _ = HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(state.clone()))
            .configure(web::routes)
    })
    .bind(("0.0.0.0", 8081))
    .unwrap()
    .run()
    .await;
    Ok(())
}
