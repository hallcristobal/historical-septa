use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::FILES_OUTPUT_DIR;
use crate::septa::{train_view::TrainView};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub id: Uuid,
    #[serde(
        deserialize_with = "crate::serde_utils::deserialize_date_time_utc",
        serialize_with = "crate::serde_utils::serialize_date_time"
    )]
    pub timestamp: DateTime<Utc>,
    pub raw: String,
    pub trains: Vec<TrainView>,
}

pub struct File {
    pub id: Uuid,
    pub received_at: DateTime<Utc>,
}

impl Content {
    pub async fn commit_file(&self, id: Uuid, pg_pool: &PgPool) -> anyhow::Result<File> {
        sqlx::query!(
            "INSERT INTO files (id, received_at) VALUES ($1, $2)",
            id,
            self.timestamp.naive_utc(),
        )
        .execute(pg_pool)
        .await?;

        {
            let contents = self.raw.clone();
            let path = format!("{}/{}.json", FILES_OUTPUT_DIR, id);
            tokio::spawn(async move {
                let mut file = match tokio::fs::File::create(&path).await {
                    Ok(file) => file,
                    Err(err) => {
                        error!("Failed to write file to file system: {:?}", err);
                        return;
                    }
                };
                match file.write_all(contents.as_bytes()).await {
                    Ok(_) => {
                        trace!("Wrote file to file system: {}", path);
                    }
                    Err(err) => {
                        error!("Failed to write file to file system: {:?}", err);
                    }
                };
            });
        }

        let file = File {
            id,
            received_at: self.timestamp,
        };
        TrainView::commit_new_records(&self.trains, &file, pg_pool).await?;

        Ok(file)
    }
}
