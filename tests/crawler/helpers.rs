use anyhow::Context;
use olx_scrapie::config::Config;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::env::var;
use uuid::Uuid;
use once_cell::sync::Lazy;

static SETUP: Lazy<()> = Lazy::new(|| {
    dotenvy::dotenv().ok();
    if let Err(e) = dotenvy::from_path(".env.testing") {
        eprintln!("Failed to load .env.testing (optional). {:?}", e);
    };
});

pub struct TestApp {
    pub config: Config,
    pub pool: PgPool,
    pub database_name: String,
    pub api_client: reqwest::Client,
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&SETUP);

    let config = Config::from_env().expect("Failed to load configuration.");

    let (database_name, pool) = configure_database().await;

    TestApp {
        config,
        pool,
        database_name,
        api_client: reqwest::Client::builder().build().unwrap(),
    }
}

async fn configure_database() -> (String, PgPool) {
    let mut database_url = var("TESTING_DATABASE_URL")
        .map(|s| url::Url::parse(&s).unwrap())
        .unwrap();

    database_url.set_path("");
    let mut connection = PgConnection::connect(database_url.as_ref())
        .await
        .expect("Failed to connect to Postgres");

    let database_name = Uuid::new_v4().to_string();
    connection
        .execute(sqlx::query(&format!(
            r#"CREATE DATABASE "{}";"#,
            &database_name
        )))
        .await
        .expect("Failed to create the DB.");

    database_url.set_path(&database_name);

    let db_pool = PgPool::connect(database_url.as_ref())
        .await
        .expect("Failed to connect to DB");

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate DB.");

    (database_name, db_pool)
}
