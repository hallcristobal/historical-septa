use std::env;

use sqlx::Connection;

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
    let mut pool = sqlx::PgConnection::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!("../migrations").run(&mut pool).await?;
    Ok(())
}
