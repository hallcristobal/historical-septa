use chrono::{DateTime, Days, Local, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::{collections::HashMap, io::ErrorKind, sync::Arc, time::Duration};
use tokio::{fs, sync::Mutex, sync::mpsc::Receiver, task::JoinHandle};

use septa::{
    db::tracking::{Fetch, Tracking},
    septa::FILES_OUTPUT_DIR,
    septa::content::Content,
    septa::train_view::TrainView,
};

pub struct AppState {
    train_statuses: Mutex<HashMap<String, Tracking<TrainView>>>,
    db_pool: PgPool,
}

impl AppState {
    pub fn new(db_pool: PgPool) -> Self {
        AppState {
            train_statuses: Mutex::new(HashMap::new()),
            db_pool,
        }
    }
}

type SharedAppState = Arc<AppState>;

pub async fn start(
    state: SharedAppState,
    file_receiver: Receiver<Content>,
) -> anyhow::Result<JoinHandle<()>> {
    ensure_directories_created().await;
    let state_handle = state.clone();
    let processer_handle = tokio::spawn(async move {
        let _ = accept_new_file(state_handle, file_receiver).await;
    });
    let _output_dir_watchdog = tokio::spawn(async move {
        let _ = schedule_file_cleanup_job().await;
    });
    Ok(processer_handle)
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
        if let Ok(most_recent_trains) = TrainView::get_most_recent_all(&state.db_pool).await {
            let most_recent_trains: HashMap<&String, &TrainView> = most_recent_trains
                .iter()
                .map(|tv| (&tv.trainno, tv))
                .collect();
            content.trains.retain(|tv| {
                if let Some(mri) = most_recent_trains.get(&tv.trainno) {
                    **mri != *tv
                } else {
                    true
                }
            });
        }

        if content.trains.is_empty() {
            // TODO: Should i drop the file if there's no "changed" trains, should i keep it but
            // just not keep a record?
            info!("File is not changed.");
            let _ = Fetch::new(content.timestamp, "UNCHANGED".to_string(), None)
                .store_fetch(&state.db_pool)
                .await;
            continue;
        }
        debug!(
            "There are {} trains changed of the {}.",
            content.trains.len(),
            incomming_len
        );

        let file_id = content.id;
        {
            let state = state.clone();
            let content = content.clone();
            tokio::spawn(async move {
                match content.commit_file(file_id, &state.db_pool).await {
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
            &mut *state.train_statuses.lock().await,
        );
        let result = json!({
            "updated": updated,
            "incomming": incomming_len,
        })
        .to_string();
        let _ = Fetch::new(content.timestamp, "OK".to_string(), Some(result))
            .store_fetch(&state.db_pool)
            .await;
        info!("Processed {len} updates. Wrote {updated}.");
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
        train_view.timestamp = *timestamp;
        if !train_statuses.contains_key(&train_view.trainno) {
            train_statuses.insert(train_view.trainno.to_owned(), Tracking::default());
        }
        let views = train_statuses.get_mut(&train_view.trainno).unwrap();
        let train_view = Arc::new(train_view);
        if *timestamp > views.most_recent_timestamp {
            if let Some(ref most_recent) = views.most_recent_item {
                let changes = train_view.get_changes(most_recent);
                views.latest_changes = changes;
                updated += 1;
            }
            views.most_recent_timestamp = *timestamp;
            views.most_recent_item = Some(train_view.clone());
        }
    });
    updated
}
