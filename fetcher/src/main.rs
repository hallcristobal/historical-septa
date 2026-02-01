#[macro_use]
extern crate log;

use actix_web::{App, HttpServer};
use sqlx::PgPool;
use std::{collections::HashMap, env, sync::Arc};
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
    let fetch_and_process_septa = !env::var("NO_FETCH").map(|v| v == "true").unwrap_or(false);

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
    Ok(())
}
