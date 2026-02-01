#[macro_use]
extern crate log;

use crate::process::processing::{self, AppState};
use septa::{
    queuing::prelude::{Opts, Queue, GenericQueue},
    septa::content::Content,
};
use std::sync::Arc;

mod process;

pub const POLL_INTERVAL: u64 = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().unwrap();
    pretty_env_logger::init_timed();

    let db_pool = septa::db::init().await.unwrap();
    let app_state = Arc::new(AppState::new(db_pool));

    let mut rabbit_queue = Queue::new(&Opts::new("localhost", 5672, "guest", "guest"))
        .await
        .unwrap();

    let (file_sender, file_receiver) = tokio::sync::mpsc::channel(100);

    rabbit_queue.ensure_queue("process_file").await.unwrap();
    let handle = processing::start(app_state, file_receiver)
        .await
        .expect("Failed to start process");

    let _ = rabbit_queue
        .consume("process_file", async |file_content: Content| {
            file_sender.send(file_content).await.unwrap();
            Ok(())
        })
        .await;

    handle.await.unwrap();
    Ok(())
}
