/// TODO: Implement this dynamically with proc_macros
use serde::Deserialize;

const DEFAULT_RESPONSE_FIELDS: [&'static str; 12] = [
    "records.id",
    "file_id",
    "trainno",
    "service",
    "dest",
    "currentstop",
    "nextstop",
    "line",
    "consist",
    "late",
    "source",
    "received_at",
];

#[derive(Default, Deserialize)]
pub struct QueryBuilder {
    pub id: Option<String>,
    pub file_id: Option<String>,
    pub timestamp: Option<i64>,
    pub trainno: Option<String>,
    pub service: Option<String>,
    pub dest: Option<String>,
    pub currentstop: Option<String>,
    pub nextstop: Option<String>,
    pub line: Option<String>,
    pub consist: Option<String>,
    pub late: Option<i32>,
    pub source: Option<String>,
    pub fields: Option<Vec<String>>,
}

#[allow(unused)]
impl QueryBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }
    pub fn with_file_id(mut self, file_id: String) -> Self {
        self.file_id = Some(file_id);
        self
    }
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
    pub fn with_trainno(mut self, trainno: String) -> Self {
        self.trainno = Some(trainno);
        self
    }
    pub fn with_service(mut self, service: String) -> Self {
        self.service = Some(service);
        self
    }
    pub fn with_dest(mut self, dest: String) -> Self {
        self.dest = Some(dest);
        self
    }
    pub fn with_currentstop(mut self, currentstop: String) -> Self {
        self.currentstop = Some(currentstop);
        self
    }
    pub fn with_nextstop(mut self, nextstop: String) -> Self {
        self.nextstop = Some(nextstop);
        self
    }
    pub fn with_line(mut self, line: String) -> Self {
        self.line = Some(line);
        self
    }
    pub fn with_consist(mut self, consist: String) -> Self {
        self.consist = Some(consist);
        self
    }
    pub fn with_late(mut self, late: i32) -> Self {
        self.late = Some(late);
        self
    }
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }
    pub fn with_fields<S: Into<String>>(mut self, fields: Vec<S>) -> Self {
        self.fields = Some(fields.into_iter().map(|s| s.into()).collect());
        self
    }
    pub fn build<'b>(self) -> sqlx::QueryBuilder<'b, sqlx::postgres::Postgres> {
        let mut builder = sqlx::QueryBuilder::new(format!(
            r#"select
{}
from
    records
"#,
            self.fields
                .unwrap_or(DEFAULT_RESPONSE_FIELDS.map(|s| s.to_owned()).to_vec())
                .join(",")
        ));
        let mut and = false;
        macro_rules! item {
            ($item:ident) => {
                if let Some($item) = self.$item {
                    if and {
                        builder.push(" and ");
                    }
                    builder.push(format!("{} = ", stringify!($item)));
                    builder.push_bind($item);
                    and = true;
                }
            };
        }
        item!(id);
        item!(file_id);
        item!(timestamp);
        item!(trainno);
        item!(service);
        item!(dest);
        item!(currentstop);
        item!(nextstop);
        item!(line);
        item!(consist);
        item!(late);
        item!(source);

        builder
    }
}
