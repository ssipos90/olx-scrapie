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
                let session = Uuid::try_parse(s).context("Failed to parse UUID")?;
                if session
                    .get_version()
                    .map_or(true, |v| v != uuid::Version::Random)
                {
                    return Err(anyhow::anyhow!("Only UUID v4 is allowed"));
                };

                if sqlx::query!(
                    r#"
                    SELECT session
                    FROM sessions
                    WHERE session=$1
                    "#,
                    session,
                )
                .fetch_optional(&app.pool)
                .await?
                .is_none()
                {
                    return Err(anyhow::anyhow!("No session found"));
                };

                session
            }
            None => {
                let session = Uuid::new_v4();

                let mut transaction = app.pool.begin().await?;
                sqlx::query!(
                    r#"
                    INSERT INTO sessions
                    (session, created_at)
                    VALUES ($1, CURRENT_TIMESTAMP)
                    "#,
                    &session
                )
                .execute(&mut transaction)
                .await?;

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

        if process_jobs(&app.pool, &session).await.is_ok() {
            let result = sqlx::query!(
                r#"
                UPDATE sessions
                SET completed_at=CURRENT_TIMESTAMP
                WHERE session=$1
                "#,
                &session
            )
            .execute(&app.pool)
            .await?;
            if result.rows_affected() == 0 {
                return Err(anyhow::anyhow!("No session has been updated lol."));
            }
        }

        Ok(())
    }
}

#[derive(clap::Subcommand)]
enum Commands {
    Play,
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
        Commands::Play => {
            use std::io::Write;
            let sleep_duration = std::time::Duration::from_secs(1);
            let mut stdout = std::io::stdout();
            for i in 1..100 {
                write!(stdout, "\x1b[2Asmf: {:?}\x1b[K\nWaa: {:?}\x1b[K\n", i, i)?;
                stdout.flush()?;

                std::thread::sleep(sleep_duration);
            }
            Ok(())
        }
    }
}
