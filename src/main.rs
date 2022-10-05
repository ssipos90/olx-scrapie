use anyhow::Context;
use clap::Parser;
use olx_scrapie::{
    config::Config,
    jobs::{insert_job, process_jobs},
    utils::PageType,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(clap::Parser)]
struct Cli {
    session: Option<String>,
}

struct App {
    pool: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let c = Config::from_env().context("Failed to create the configuration.")?;

    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    // let update_assets = args.update_assets.unwrap_or(false);

    let app = App {
        pool: sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy(c.database_url.as_ref())
            .context("Failed to connect to postgres.")?,
    };

    let session = match args.session {
        Some(s) => {
            let uuid = Uuid::try_parse(&s).context("Failed to parse UUID")?;
            match uuid.get_version() {
                Some(uuid::Version::Random) => uuid,
                _ => panic!("Only UUID v4 is allowed"),
            }
        }
        None => {
            let session = Uuid::new_v4();
            let mut transaction = app.pool.begin().await?;
            insert_job(
                &mut transaction,
                &session,
                &c.list_page_url,
                PageType::OlxList,
            )
            .await?;
            transaction.commit().await?;
            session
        },
    };

    process_jobs(&app.pool, &session).await;

    Ok(())
}
