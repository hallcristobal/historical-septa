#[macro_use]
extern crate log;

use septa::{
    queuing::prelude::{Opts, Queue, GenericQueue},
    septa::content::Content,
};
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};

pub const POLL_INTERVAL: u64 = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().unwrap();
    pretty_env_logger::init_timed();
    info!("Starting Septa processes");
    let mut rabbit_queue = Queue::new(&Opts::new("localhost", 5672, "guest", "guest"))
        .await
        .unwrap();
    rabbit_queue.ensure_exchange("process_file").await.unwrap();

    let (file_sender, file_receiver) = tokio::sync::mpsc::channel(10);
    let poll_handle = tokio::spawn(async move {
        let _ = poll_for_train_view(POLL_INTERVAL, file_sender).await;
    });

    let processer_handle = tokio::spawn(async move {
        let _ = forward_file(rabbit_queue, file_receiver).await;
    });

    poll_handle.await.unwrap();
    processer_handle.await.unwrap();

    Ok(())
}

pub async fn poll_for_train_view(interval: u64, sender: Sender<Content>) {
    let sleep_duration = Duration::from_secs(interval);
    loop {
        match septa::septa::api::fetch_train_view().await {
            Ok(content) => {
                if let Err(e) = sender.send(content).await {
                    error!("Sender failed: {e:?}");
                    break;
                }
            }
            Err(e) => {
                error!("Failed to fetch file: {:?}", e);
                todo!("On fetch error, save error to db.")
                // let _ = Fetch::new(e.0, "FETCH_ERROR".to_string(), Some(e.1))
                //     .store_fetch(state.read().await.pg_pool.clone())
                //     .await;
            }
        }
        tokio::time::sleep(sleep_duration).await;
    }
}

pub async fn forward_file(mut queue: Queue, mut receiver: Receiver<Content>) {
    while let Some(content) = receiver.recv().await {
        debug!("Received file for processing: {:?}", content.id);
        let incomming_len = content.trains.len();
        debug!("File has {} trains listed.", incomming_len);
        queue.send("process_file", content).await.unwrap();
    }
}
