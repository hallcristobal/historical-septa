#[macro_use]
extern crate log;

use aws_config::BehaviorVersion;
use aws_credential_types::provider::ProvideCredentials;
use aws_sigv4::{
    http_request::{SignableBody, SignableRequest, SigningSettings, sign},
    sign::v4,
};
use lambda_http::{Body, Error, Request, Response, run, service_fn};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};
use std::{env, sync::Arc};

use septa::{
    db::tracking::Tracking,
    septa::{
        query_builder::QueryBuilder,
        train_view::{TrainView, enforce_limit_bounds},
    },
};

use sqlx::{PgPool, postgres::PgConnectOptions};
use tokio::sync::RwLock;

struct AppState {
    pg_pool: PgPool,
}
type SharedAppState = Arc<RwLock<AppState>>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init_timed();
    run(service_fn(handler)).await
}

async fn handler(_evnet: Request) -> Result<Response<Body>, Error> {
    let db_host = env::var("DATABASE_HOST").expect("DB_HOSTNAME must be set");
    let db_port = env::var("DATABASE_PORT")
        .expect("DATABASE_PORT must be set")
        .parse::<u16>()
        .expect("PORT must be a valid number");
    let db_pass = env::var("DATABASE_PASS").expect("DB_PASS must be set");
    let db_name = env::var("DATABASE_NAME").expect("DB_NAME must be set");
    let db_user_name = env::var("DATABASE_USER").expect("DB_USERNAME must be set");

    let state = AppState {
        pg_pool: connect_db(&db_host, db_port, &db_user_name, &db_name, &db_pass).await?,
    };
    info!("Connected to database at: {:?}", state.pg_pool);
    let state = Arc::new(RwLock::new(state));
    let query = GetCurrentQuery::default();

    match current_trains(query, state).await {
        Ok(res) => {
            info!("Responding with result: {:?}", res);
            let res = serde_json::to_string(&res).map_err(Box::new)?;
            let res = Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(res.into())
                .map_err(Box::new)?;
            Ok(res)
        }
        Err(err) => {
            error!("Error when querying: {:?}", err);
            Err(err)
        }
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct GetCurrentQuery {
    all: Option<bool>,
    line: Option<String>,
    limit: Option<i64>,
}

async fn current_trains(query: GetCurrentQuery, data: SharedAppState) -> Result<Value, Error> {
    let two_am_today = chrono::Local::now()
        .with_time(chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap())
        .unwrap()
        .to_utc();
    let count = enforce_limit_bounds(query.limit);
    let all = query.all.unwrap_or(false);
    let line = query.line.as_ref();
    let most_recent = get_most_recents(data).await?;
    let recent = most_recent
        .iter()
        .filter_map(|tv| {
            if let Some(ref mri) = tv.1.most_recent_item {
                if let Some(line) = line
                    && *line != mri.line
                {
                    return None;
                }

                if all || mri.timestamp > two_am_today {
                    Some(mri.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .take(count as usize)
        .collect::<Vec<Arc<TrainView>>>();
    debug!("Received response: {:?}", recent);

    #[derive(Serialize)]
    struct Response {
        count: u32,
        statuses: Vec<Arc<TrainView>>,
    }

    let response = json!(Response {
        count: recent.len() as u32,
        statuses: recent,
    });
    Ok(response)
}

const RDS_CERTS: &[u8] = include_bytes!("global-bundle.pem");

async fn connect_db(
    db_host: &str,
    db_port: u16,
    db_user_name: &str,
    db_name: &str,
    db_pass: &str,
) -> Result<PgPool, Error> {
    // let token = generate_rds_iam_token(db_host, db_port, db_user_name).await?;
    debug!(
        "Trying to connect to PgDatabase... {:?}",
        env::var("DATABASE_URL")
    );
    // db::init().await.map_err(Error::from)

    let mut opts = PgConnectOptions::new()
        .host(db_host)
        .port(db_port)
        .username(db_user_name)
        .password(db_pass)
        .database(db_name)
        .ssl_root_cert_from_pem(RDS_CERTS.to_vec())
        .ssl_mode(sqlx::postgres::PgSslMode::VerifyFull);
    if let Ok(local) = env::var("IS_LOCAL") {
        info!("local variable found: {}", local);
        if &local == "true" {
            info!("Using local config!");
            opts = opts
                .ssl_root_cert_from_pem(vec![])
                .ssl_mode(sqlx::postgres::PgSslMode::Require);
        }
    }

    trace!("Trying to connect to PgDatabase... {}", opts.get_host());

    sqlx::postgres::PgPoolOptions::new()
        .connect_with(opts)
        .await
        .map_err(Error::from)
}

#[allow(unused)]
async fn generate_rds_iam_token(
    db_hostname: &str,
    port: u16,
    db_username: &str,
) -> Result<String, Error> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;

    let credentials = config
        .credentials_provider()
        .expect("no credentials provider found")
        .provide_credentials()
        .await
        .expect("unable to load credentials");
    let identity = credentials.into();
    let region = config.region().unwrap().to_string();

    let mut signing_settings = SigningSettings::default();
    signing_settings.expires_in = Some(Duration::from_secs(900));
    signing_settings.signature_location = aws_sigv4::http_request::SignatureLocation::QueryParams;

    let signing_params = v4::SigningParams::builder()
        .identity(&identity)
        .region(&region)
        .name("rds-db")
        .time(SystemTime::now())
        .settings(signing_settings)
        .build()?;

    let url = format!("https://{db_hostname}:{port}/?Action=connect&DBUser={db_username}");

    let signable_request =
        SignableRequest::new("GET", &url, std::iter::empty(), SignableBody::Bytes(&[]))
            .expect("signable request");

    let (signing_instructions, _signature) =
        sign(signable_request, &signing_params.into())?.into_parts();

    let mut url = url::Url::parse(&url).unwrap();
    for (name, value) in signing_instructions.params() {
        url.query_pairs_mut().append_pair(name, value);
    }

    let response = url.to_string().split_off("https://".len());

    Ok(response)
}

async fn get_most_recents(
    state: SharedAppState,
) -> anyhow::Result<HashMap<String, Tracking<TrainView>>> {
    let query = QueryBuilder::new().with_line("Lansdale/Doylestown".into());
    let train_views = TrainView::query_trains(
        state.read().await.pg_pool.clone(),
        query,
        Some(10),
        None,
        None,
        None,
    )
    .await?;
    let mut train_statuses: HashMap<String, Tracking<TrainView>> = HashMap::new();
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
    Ok(train_statuses)
}
