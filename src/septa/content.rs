use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::septa::{processing::FILES_OUTPUT_DIR, train_view::TrainView};

#[derive(Debug, Clone)]
pub struct Content {
    pub timestamp: DateTime<Utc>,
    pub raw: String,
    pub trains: Vec<TrainView>,
}

pub struct File {
    pub id: Uuid,
    pub received_at: DateTime<Utc>,
}

impl Content {
    pub async fn commit_file(&self, id: Uuid, pg_pool: PgPool) -> anyhow::Result<File> {
        sqlx::query!(
            "INSERT INTO files (id, received_at) VALUES ($1, $2)",
            id,
            self.timestamp.naive_utc(),
        )
        .execute(&pg_pool)
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
                        return;
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
