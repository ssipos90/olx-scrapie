use anyhow::Context;
use clap::Parser;
use olx_scrapie::{
    config::Config,
    jobs::{insert_job, process_jobs},
    page::PageType,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Args)]
struct DownloadCmd {
    session: Option<String>,
}

impl DownloadCmd {
    async fn work(&self, cfg: &Config, app: &App) -> anyhow::Result<()> {
        let session = match &self.session {
            Some(s) => {
                let uuid = Uuid::try_parse(s).context("Failed to parse UUID")?;
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
                    &cfg.list_page_url,
                    PageType::OlxList,
                )
                .await?;
                transaction.commit().await?;
                session
            }
        };

        process_jobs(&app.pool, &session).await;

        Ok(())
    }
}

#[derive(clap::Subcommand)]
enum Commands {
    Download(DownloadCmd),
}

struct App {
    pool: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cfg = Config::from_env().context("Failed to load the configuration.")?;

    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    // let update_assets = args.update_assets.unwrap_or(false);

    let app = App {
        pool: sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy(cfg.database_url.as_ref())
            .context("Failed to establish lazy connection to postgres.")?,
    };

    match args.command {
        Commands::Download(cmd) => cmd.work(&cfg, &app).await,
    }
}
