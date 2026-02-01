#[macro_use]
extern crate log;

use crate::process::processing::{self, AppState};
use std::sync::Arc;

mod process;

pub const POLL_INTERVAL: u64 = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().unwrap();
    pretty_env_logger::init_timed();

    let db_pool = septa::db::init().await.unwrap();
    let app_state = Arc::new(AppState::new(db_pool));

    let (_, file_receiver) = tokio::sync::mpsc::channel(100);
    let handle = processing::start(app_state, file_receiver)
        .await
        .expect("Failed to start process");
    handle.await.unwrap();
    Ok(())
}
