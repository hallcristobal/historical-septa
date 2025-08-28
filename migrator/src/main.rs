use sqlx::Connection;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().unwrap();
    let mut pool = sqlx::PgConnection::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!("../migrations").run(&mut pool).await?;
    Ok(())
}
