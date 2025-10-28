use sqlx::{Connection, postgres::PgConnectOptions};
use std::env;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().unwrap();
    unsafe {
        env::set_var(
            "RUST_LOG",
            match env::var("RUST_LOG") {
                Ok(env_level) => format!("{env_level},sqlx=TRACE"),
                Err(_) => String::from("sqlx=TRACE"),
            },
        );
    }

    pretty_env_logger::init_timed();
    let opts: PgConnectOptions = dotenvy::var("DATABASE_URL")?.parse()?;
    let opts = if opts.get_host() != "127.0.0.1" && opts.get_host() != "localhost" {
        opts.ssl_mode(sqlx::postgres::PgSslMode::Require)
    } else {
        opts
    };
    let mut pool = sqlx::PgConnection::connect_with(&opts).await?;
    if let Err(err) = sqlx::migrate!("../migrations").run(&mut pool).await {
        eprintln!("Error running migration: {}", err);
    }
    Ok(())
}
