use crate::{AppState, log_current_cared_statuses, process_train_view, septa::train_view::Content};
use chrono::{DateTime, TimeZone, Utc};
use std::{fs, io::BufReader};

pub fn test_local(state: &mut AppState) -> std::io::Result<()> {
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
        let timestamp: DateTime<Utc> = Utc.timestamp_opt(timestamp, 0).unwrap();
        let reader = BufReader::new(fs::File::open(path).unwrap());
        let content = serde_json::from_reader::<_, Content>(reader)
            .map_err(|err| std::io::Error::other(format!("Decode Error: {err}")));
        if let Ok(content) = content {
            content
                .0
                .into_iter()
                .for_each(|tv| process_train_view(tv, &timestamp, &mut state.train_statuses));
        }
    });

    println!("keys: {:?}", state.train_statuses.keys().len());
    log_current_cared_statuses(&state.train_statuses);
    Ok(())
}
