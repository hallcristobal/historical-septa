use chrono::{DateTime, Local, TimeZone};
use std::{collections::HashMap, fs, io::BufReader, rc::Rc};
use tracking::Tracking;

use crate::septa::train_view::{Content, TrainView};

mod septa;
mod serde_utils;
mod tracking;

fn main() -> std::io::Result<()> {
    let mut train_statuses: HashMap<String, Tracking<TrainView>> = HashMap::new();
    let mut entries = fs::read_dir("./responses")
        .expect("Unable to read responses directory")
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .expect("Failed to read responses directory");

    entries.sort();

    entries.into_iter().for_each(|path| {
        let timestamp: i64 = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(".json", "")
            .parse()
            .unwrap();
        let timestamp: DateTime<Local> = Local.timestamp_opt(timestamp, 0).unwrap();
        let reader = BufReader::new(fs::File::open(path).unwrap());
        let content = serde_json::from_reader::<_, Content>(reader)
            .map_err(|err| std::io::Error::other(format!("Decode Error: {err}")));
        if let Ok(content) = content {
            content
                .0
                .into_iter()
                .for_each(|tv| process_train_view(tv, &timestamp, &mut train_statuses));
            has_train_vanished()
        }
    });

    println!("keys: {:?}", train_statuses.keys().len());
    log_current_cared_statuses(&train_statuses);
    Ok(())
}
fn process_train_view(
    mut train_view: TrainView,
    timestamp: &DateTime<Local>,
    train_statuses: &mut HashMap<String, Tracking<TrainView>>,
) {
    train_view.timestamp = timestamp.clone();
    if !train_statuses.contains_key(&train_view.trainno) {
        train_statuses.insert(train_view.trainno.to_owned(), Tracking::default());
    }
    let views = train_statuses.get_mut(&train_view.trainno).unwrap();
    let train_view = Rc::new(train_view);
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
