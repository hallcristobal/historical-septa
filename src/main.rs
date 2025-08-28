use actix_web::{
    App, HttpServer, Responder,
    web::{self, Json},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{
    RwLock,
    mpsc::{Receiver, Sender},
};
use tracking::Tracking;

use crate::{
    septa::{
        FetchResult,
        train_view::{Content, TrainView},
    },
    tracking::Changed,
};

mod septa;
mod serde_utils;
mod testing;
mod tracking;

struct AppState {
    train_statuses: HashMap<String, Tracking<TrainView>>,
}
type SharedAppState = Arc<RwLock<AppState>>;

async fn accept_new_file(state: SharedAppState, mut recv: Receiver<FetchResult<Content>>) {
    while let Some((timestamp, content)) = recv.recv().await {
        let len = content.0.len();
        let updated = process_train_views(
            content.0,
            &timestamp,
            &mut state.write().await.train_statuses,
        );
        println!("Processed {len} updates. Wrote {updated}.");
    }
}

async fn poll_for_train_view(
    _state: SharedAppState,
    interval: u64,
    sender: Sender<FetchResult<Content>>,
) {
    let sleep_duration = Duration::from_secs(interval);
    while let Ok(content) = septa::api::fetch_train_view().await {
        match sender.send(content).await {
            Err(e) => {
                eprintln!("Sender failed: {e:?}");
                break;
            }
            _ => {}
        }
        tokio::time::sleep(sleep_duration).await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState {
        train_statuses: HashMap::new(),
    };
    let state = Arc::new(RwLock::new(state));
    let state_handle = state.clone();
    let (file_sender, file_receiver) = tokio::sync::mpsc::channel(1);
    let poll_handle = tokio::spawn(async move {
        let _ = poll_for_train_view(state_handle, 5, file_sender).await;
    });

    let state_handle = state.clone();
    let processer_handle = tokio::spawn(async move {
        let _ = accept_new_file(state_handle, file_receiver).await;
    });

    let _ = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .configure(routes)
    })
    .bind(("0.0.0.0", 8081))
    .unwrap()
    .run()
    .await;
    let _ = poll_handle.await;
    let _ = processer_handle.await;
    Ok(())
}

async fn most_recent_trains(data: web::Data<SharedAppState>) -> impl Responder {
    let recent = data
        .read()
        .await
        .train_statuses
        .iter()
        .filter_map(|tv| tv.1.most_recent_item.clone())
        .collect::<Vec<Arc<TrainView>>>();
    #[derive(Serialize)]
    struct Response {
        statuses: Vec<Arc<TrainView>>,
    }
    Json(Response { statuses: recent })
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
                    .filter(|c| c.timestamp >= until)
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

fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/most_recent", web::get().to(most_recent_trains))
            .route("/recent_changes", web::get().to(most_recent_changes)),
    );
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
                if let Some(changes) = train_view.has_changed(&most_recent) {
                    views.latest_changes = Some(changes);
                    updated += 1;
                }
            }
            views.most_recent_timestamp = *timestamp;
            views.most_recent_item = Some(train_view.clone());
        }
        views.items.push(train_view);
    });
    updated
}

fn process_train_view(
    mut train_view: TrainView,
    timestamp: &DateTime<Utc>,
    train_statuses: &mut HashMap<String, Tracking<TrainView>>,
) {
    train_view.timestamp = timestamp.clone();
    if !train_statuses.contains_key(&train_view.trainno) {
        train_statuses.insert(train_view.trainno.to_owned(), Tracking::default());
    }
    let views = train_statuses.get_mut(&train_view.trainno).unwrap();
    let train_view = Arc::new(train_view);
    if *timestamp > views.most_recent_timestamp {
        if let Some(ref most_recent) = views.most_recent_item {
            if let Some(changes) = train_view.has_changed(&most_recent) {
                views.latest_changes = Some(changes);
            }
        }
        views.most_recent_timestamp = *timestamp;
        views.most_recent_item = Some(train_view.clone());
    }
    views.items.push(train_view);
}

fn log_current_cared_statuses(statuses: &HashMap<String, Tracking<TrainView>>) {
    statuses.iter().for_each(|(trainno, tracking)| {
        if let Some(ref item) = tracking.most_recent_item {
            if item.late > 3 {
                println!("Train {} is running {} minutes late.", trainno, item.late);
            }
        }
    });
}
