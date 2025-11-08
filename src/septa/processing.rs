use super::api;
use chrono::{DateTime, Days, Local, Utc};
use serde_json::json;
use std::{collections::HashMap, io::ErrorKind, sync::Arc, time::Duration};
use tokio::{
    fs,
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    SharedAppState,
    db::tracking::{Fetch, Tracking},
    septa::content::Content,
    septa::train_view::TrainView,
};

pub const FILES_OUTPUT_DIR: &'static str = "./files";
pub const POLL_INTERVAL: u64 = 5;

pub async fn start(state: SharedAppState) -> anyhow::Result<(JoinHandle<()>, JoinHandle<()>)> {
    let state_handle = state.clone();
    let (file_sender, file_receiver) = tokio::sync::mpsc::channel(1);
    ensure_directories_created().await;
    let poll_handle = tokio::spawn(async move {
        let _ = poll_for_train_view(state_handle, POLL_INTERVAL, file_sender).await;
    });

    let state_handle = state.clone();
    let processer_handle = tokio::spawn(async move {
        let _ = accept_new_file(state_handle, file_receiver).await;
    });
    let _output_dir_watchdog = tokio::spawn(async move {
        let _ = schedule_file_cleanup_job().await;
    });
    Ok((poll_handle, processer_handle))
}

pub async fn ensure_directories_created() {
    match tokio::fs::create_dir(FILES_OUTPUT_DIR).await {
        Ok(_) => {
            warn!("Output directory created.");
            Ok(())
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            warn!("Output directory already exists. Nothing to do.");
            Ok(())
        }
        Err(err) => Err(err),
    }
    .expect("Unable to create output directory");
}

pub async fn accept_new_file(state: SharedAppState, mut recv: Receiver<Content>) {
    while let Some(mut content) = recv.recv().await {
        let incomming_len = content.trains.len();
        {
            let statuses = &state.read().await.train_statuses;
            content.trains.retain(|tv| {
                if let Some(existing) = statuses.get(&tv.trainno) {
                    if let Some(ref mri) = existing.most_recent_item {
                        if **mri != *tv {
                            return true;
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                return true;
            });
        }

        if content.trains.len() == 0 {
            // TODO: Should i drop the file if there's no "changed" trains, should i keep it but
            // just not keep a record?
            info!("File is not changed.");
            let _ = Fetch::new(content.timestamp, "UNCHANGED".to_string(), None)
                .store_fetch(state.read().await.pg_pool.clone())
                .await;
            continue;
        }
        debug!(
            "There are {} trains changed of the {}.",
            content.trains.len(),
            incomming_len
        );

        let file_id = uuid::Uuid::new_v4();
        {
            let state = state.clone();
            let content = content.clone();
            let file_id = file_id.clone();
            tokio::spawn(async move {
                match content
                    .commit_file(file_id, state.read().await.pg_pool.clone())
                    .await
                {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Failed to execute commit_file: {:?}", err);
                    }
                }
            });
        }
        let len = content.trains.len();
        content
            .trains
            .iter_mut()
            .for_each(|tv| tv.file_id = file_id);

        let updated = process_train_views(
            content.trains,
            &content.timestamp,
            &mut state.write().await.train_statuses,
        );
        let result = json!({
            "updated": updated,
            "incomming": incomming_len,
        })
        .to_string();
        let _ = Fetch::new(content.timestamp, "OK".to_string(), Some(result))
            .store_fetch(state.read().await.pg_pool.clone())
            .await;
        info!("Processed {len} updates. Wrote {updated}.");
    }
}

pub async fn poll_for_train_view(state: SharedAppState, interval: u64, sender: Sender<Content>) {
    let sleep_duration = Duration::from_secs(interval);
    loop {
        match api::fetch_train_view().await {
            Ok(content) => match sender.send(content).await {
                Err(e) => {
                    error!("Sender failed: {e:?}");
                    break;
                }
                _ => {}
            },
            Err(e) => {
                let _ = Fetch::new(e.0, "FETCH_ERROR".to_string(), Some(e.1))
                    .store_fetch(state.read().await.pg_pool.clone())
                    .await;
            }
        }
        tokio::time::sleep(sleep_duration).await;
    }
}

pub async fn schedule_file_cleanup_job() {
    let sleep_duration = Duration::from_secs(60 * 60); // 1 Hour
    info!(
        "Started file cleanup watchdog, scheduled to run every {} seconds. ",
        sleep_duration.as_secs()
    );
    loop {
        info!("Starting file cleanup task.");
        let mut removed = 0;
        let last_week = chrono::Local::now().checked_sub_days(Days::new(7)).unwrap();
        match fs::read_dir(FILES_OUTPUT_DIR).await {
            Ok(mut files) => {
                while let Ok(Some(file)) = files.next_entry().await {
                    match file.metadata().await.and_then(|meta| meta.modified()) {
                        Ok(btime) => {
                            let created_time = chrono::DateTime::<Local>::from(btime);
                            if created_time < last_week {
                                removed += 1;
                                let path = file.path().clone();
                                tokio::spawn(async move {
                                    if let Err(e) = fs::remove_file(&path).await {
                                        error!("Failed to remove file: {:?} - {:?}", path, e);
                                    }
                                });
                            }
                        }
                        Err(e) => {
                            error!("Failed to get the metadata for file: {:?} - {:?}", file, e);
                        }
                    }
                }
            }
            Err(e) => error!("Error reading directory: {e:?}"),
        }
        info!("File cleanup task completed. Removed: {} files.", removed);
        tokio::time::sleep(sleep_duration).await;
    }
}

fn process_train_views(
    train_views: Vec<TrainView>,
    timestamp: &DateTime<Utc>,
    train_statuses: &mut HashMap<String, Tracking<TrainView>>,
) -> usize {
    let mut updated = 0;
    train_views.into_iter().for_each(|mut train_view| {
        train_view.timestamp = timestamp.clone();
        if !train_statuses.contains_key(&train_view.trainno) {
            train_statuses.insert(train_view.trainno.to_owned(), Tracking::default());
        }
        let views = train_statuses.get_mut(&train_view.trainno).unwrap();
        let train_view = Arc::new(train_view);
        if *timestamp > views.most_recent_timestamp {
            if let Some(ref most_recent) = views.most_recent_item {
                let changes = train_view.get_changes(&most_recent);
                views.latest_changes = changes;
                updated += 1;
            }
            views.most_recent_timestamp = *timestamp;
            views.most_recent_item = Some(train_view.clone());
        }
        // views.items.push(train_view);
    });
    updated
}
