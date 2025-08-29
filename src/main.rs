#[macro_use]
extern crate log;

use actix_web::{
    App, HttpServer, Responder,
    web::{self, Json},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::{collections::HashMap, env, sync::Arc, time::Duration};
use tokio::sync::{
    RwLock,
    mpsc::{Receiver, Sender},
};
use tracking::Tracking;

use crate::{
    db::QueryOrdering,
    septa::train_view::{Content, TrainView},
    tracking::{Changed, Fetch},
};

mod db;
mod septa;
mod serde_utils;
mod testing;
mod tracking;

struct AppState {
    train_statuses: HashMap<String, Tracking<TrainView>>,
    pg_pool: PgPool,
}
type SharedAppState = Arc<RwLock<AppState>>;

async fn accept_new_file(state: SharedAppState, mut recv: Receiver<Content>) {
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
                let _ = content
                    .commit_file(file_id, state.read().await.pg_pool.clone())
                    .await;
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

async fn poll_for_train_view(state: SharedAppState, interval: u64, sender: Sender<Content>) {
    let sleep_duration = Duration::from_secs(interval);
    loop {
        match septa::api::fetch_train_view().await {
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
    println!("trace");
    pretty_env_logger::init_timed();
    let state = AppState {
        train_statuses: HashMap::new(),
        pg_pool: db::init().await.unwrap(),
    };
    let state = Arc::new(RwLock::new(state));
    let backfilled = populate_known_statuses(state.clone()).await?;
    info!("Backfilled {} statuses during startup.", backfilled);
    let state_handle = state.clone();
    let (file_sender, file_receiver) = tokio::sync::mpsc::channel(1);
    let _poll_handle = tokio::spawn(async move {
        let _ = poll_for_train_view(state_handle, 5, file_sender).await;
    });

    let state_handle = state.clone();
    let _processer_handle = tokio::spawn(async move {
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
                    .filter(|c| c.changed_at >= until)
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
            .route("/train/{id}", web::get().to(get_train))
            .route("/recent_changes", web::get().to(most_recent_changes)),
    );
}

#[derive(Deserialize)]
struct GetTrainPath {
    id: String,
}
#[derive(Deserialize)]
struct GetTrainQuery {
    limit: Option<i64>,
    before: Option<i64>,
    after: Option<i64>,
    order: Option<QueryOrdering>,
}
async fn get_train(
    path: web::Path<GetTrainPath>,
    query: web::Query<GetTrainQuery>,
    data: web::Data<SharedAppState>,
) -> impl Responder {
    #[derive(Serialize)]
    struct Response {
        records: Vec<TrainView>,
    }
    let pg_pool = data.read().await.pg_pool.clone();

    match TrainView::fetch_for_train(
        pg_pool,
        &path.id,
        query.limit,
        query.before.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        query.after.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        query.order,
    )
    .await
    {
        Ok(records) => Json(Response { records }),
        Err(e) => {
            error!("Error fetching: {e}");
            Json(Response {
                records: Vec::new(),
            })
        }
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
            if let Some(changes) = train_view.get_changes(&most_recent) {
                views.latest_changes = Some(changes);
            }
        }
        views.most_recent_timestamp = *timestamp;
        views.most_recent_item = Some(train_view.clone());
    }
    // views.items.push(train_view);
}

fn log_current_cared_statuses(statuses: &HashMap<String, Tracking<TrainView>>) {
    statuses.iter().for_each(|(trainno, tracking)| {
        if let Some(ref item) = tracking.most_recent_item {
            if item.late > 3 {
                info!("Train {} is running {} minutes late.", trainno, item.late);
            }
        }
    });
}
