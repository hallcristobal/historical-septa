use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Database, PgPool, QueryBuilder, Row, query_builder};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{db::{
    tracking::{Changed, Value}, QueryOrdering
}, septa::processing::FILES_OUTPUT_DIR};

#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
pub struct TrainView {
    #[serde(skip_deserializing, default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(skip_deserializing, default)]
    pub file_id: Uuid,
    #[serde(
        skip_deserializing,
        default,
        serialize_with = "crate::serde_utils::serialize_date_time"
    )]
    pub timestamp: chrono::DateTime<Utc>,
    // #[serde(deserialize_with = "crate::serde_utils::deserialize_f32_string")]
    // pub lat: f32,
    // #[serde(deserialize_with = "crate::serde_utils::deserialize_f32_string")]
    // pub lon: f32,
    pub trainno: String,
    pub service: String,
    pub dest: String,
    pub currentstop: String,
    pub nextstop: String,
    pub line: String,
    pub consist: String,
    // #[serde(deserialize_with = "crate::serde_utils::deserialize_opt_f32_string")]
    // pub heading: Option<f32>,
    pub late: i32,
    #[serde(rename = "SOURCE")]
    pub source: String,
    // #[serde(rename = "TRACK")]
    // pub track: String,
    // #[serde(rename = "TRACK_CHANGE")]
    // pub track_change: String,
}

impl PartialEq for TrainView {
    fn eq(&self, other: &Self) -> bool {
        self.trainno == other.trainno
            && self.service == other.service
            && self.dest == other.dest
            && self.currentstop == other.currentstop
            && self.nextstop == other.nextstop
            && self.line == other.line
            && self.consist == other.consist
            && self.late == other.late
            && self.source == other.source
    }
}

macro_rules! evaluate_changes {
    ($si:ident, $self:ident, $prev:ident, $type:ident, $store:ident) => {
        if $self.$si != $prev.$si {
            let new_value = Value::$type($self.$si.clone());
            $store.push(Changed {
                id: Uuid::new_v4(),
                trainno: $self.trainno.clone(),
                record_id: $self.id,
                changed_at: $self.timestamp,
                field: String::from(stringify!($si)),
                old_value: Value::$type($prev.$si.clone()),
                new_value: new_value.clone(),
                _type: new_value.string_name(),
            });
        }
    };
    ([$(($si:ident, $type:ident)),*], $self:ident, $prev:ident, $store:ident) => {
        $( evaluate_changes!($si, $self, $prev, $type, $store); )*
    };
}

impl TrainView {
    pub fn get_changes(&self, prev: &TrainView) -> Option<Vec<Changed>> {
        let mut changed = vec![];
        if self.trainno != prev.trainno {
            return None;
        }

        evaluate_changes!(
            [
                (service, String),
                (late, Int),
                (trainno, String),
                (dest, String),
                (currentstop, String),
                (nextstop, String),
                (line, String),
                (consist, String),
                (source, String)
            ],
            self,
            prev,
            changed
        );

        if changed.len() > 0 {
            Some(changed)
        } else {
            None
        }
    }

    /// Database
    pub async fn get_most_recent_all(pool: PgPool) -> anyhow::Result<Vec<TrainView>> {
        let records = sqlx::query!(
            r"
select 
  distinct on (trainno) 
  records.id,
  file_id,
  trainno,
  service,
  dest,
  currentstop,
  nextstop,
  line,
  consist,
  late,
  source,
  received_at
from 
     records 
order by 
    trainno, 
    received_at desc
"
        )
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|row| TrainView {
            id: row.id,
            file_id: row.file_id,
            timestamp: row
                .received_at
                .and_then(|r| Some(r.and_utc()))
                .unwrap_or_default(),
            trainno: row.trainno.clone(),
            service: row.service.clone(),
            dest: row.dest.clone(),
            currentstop: row.currentstop.clone(),
            nextstop: row.nextstop.clone(),
            line: row.line.clone(),
            consist: row.consist.clone(),
            late: row.late.clone(),
            source: row.source.clone(),
        })
        .collect();
        Ok(records)
    }
    pub async fn fetch_for_train(
        pool: PgPool,
        trainno: &str,
        limit: Option<i64>,
        before: Option<DateTime<Utc>>,
        after: Option<DateTime<Utc>>,
        order: Option<QueryOrdering>,
    ) -> anyhow::Result<Vec<TrainView>> {
        let mut builder = query_builder::QueryBuilder::new(
            r#"select
  records.id,
  file_id,
  trainno,
  service,
  dest,
  currentstop,
  nextstop,
  line,
  consist,
  late,
  source,
  received_at
from
    records
"#,
        );
        fn where_helper<'args, T, DB: Database>(
            field: &str,
            comparator: &str,
            val: T,
            builder: &mut sqlx::QueryBuilder<'args, DB>,
        ) where
            T: 'args + sqlx::Encode<'args, DB> + sqlx::Type<DB>,
        {
            builder.push(format!(" and {} {} ", field, comparator));
            builder.push_bind(val);
        }

        builder.push(" WHERE trainno = ");
        builder.push_bind(trainno);

        if let Some(before) = before {
            where_helper("received_at", "<", before, &mut builder);
        }
        if let Some(after) = after {
            where_helper("received_at", ">", after, &mut builder);
        }

        if let Some(order) = order {
            // TODO: This is unsafe, but it's the best way to do it since it's an enum, and we
            // alredy throw an error if it's the incorrect of the two options anywa...
            builder.push(format!(" ORDER BY received_at {}", order));
        }
        if let Some(limit) = limit {
            builder.push(" LIMIT ");
            builder.push_bind(limit);
        }

        let results = builder.build();
        let results = results.fetch_all(&pool).await?;

        let records: Vec<TrainView> = results
            .iter()
            .map(|row| TrainView {
                id: row.get("id"),
                file_id: row.get("file_id"),
                timestamp: row.get::<NaiveDateTime, &str>("received_at").and_utc(),
                trainno: row.get("trainno"),
                service: row.get("service"),
                dest: row.get("dest"),
                currentstop: row.get("currentstop"),
                nextstop: row.get("nextstop"),
                line: row.get("line"),
                consist: row.get("consist"),
                late: row.get("late"),
                source: row.get("source"),
            })
            .collect();
        Ok(records)
    }

    #[allow(unused)]
    pub async fn commit_new_record(&self, file: &File, pg_pool: PgPool) -> anyhow::Result<()> {
        sqlx::query!(
            r" INSERT INTO records 
    (id, file_id, received_at, trainno, service, dest, currentstop, nextstop, line, consist, late, source)
VALUES
    ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            self.id,
            file.id,
            file.received_at.naive_utc(),
            self.trainno,
            self.service,
            self.dest,
            self.currentstop,
            self.nextstop,
            self.line,
            self.consist,
            self.late,
            self.source,
        )
        .execute(&pg_pool)
        .await?;
        Ok(())
    }

    pub async fn commit_new_records(
        records: &Vec<TrainView>,
        file: &File,
        pg_pool: PgPool,
    ) -> anyhow::Result<u64> {
        let mut builder = QueryBuilder::new(
            r" INSERT INTO records 
    (id, file_id, received_at, trainno, service, dest, currentstop, nextstop, line, consist, late, source) ",
        );
        builder.push_values(records.iter(), |mut a, record| {
            a.push_bind(&record.id)
                .push_bind(&file.id)
                .push_bind(file.received_at.naive_utc())
                .push_bind(&record.trainno)
                .push_bind(&record.service)
                .push_bind(&record.dest)
                .push_bind(&record.currentstop)
                .push_bind(&record.nextstop)
                .push_bind(&record.line)
                .push_bind(&record.consist)
                .push_bind(&record.late)
                .push_bind(&record.source);
        });
        let inserted = builder.build().execute(&pg_pool).await?;
        Ok(inserted.rows_affected())
    }
}

#[derive(Debug, Clone)]
pub struct Content {
    pub timestamp: DateTime<Utc>,
    pub raw: String,
    pub trains: Vec<TrainView>,
}

pub struct File {
    id: Uuid,
    received_at: DateTime<Utc>,
    // contents: String,
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
            // contents: self.raw.to_owned(),
        };
        TrainView::commit_new_records(&self.trains, &file, pg_pool).await?;

        Ok(file)
    }
}
