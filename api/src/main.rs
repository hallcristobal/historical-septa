#[macro_use]
extern crate log;

mod web;

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
