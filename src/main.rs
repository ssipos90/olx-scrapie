use anyhow::Context;
use clap::Parser;
use olx_scrapie::{
    config::Config,
    utils::{get_list_next_page_url, save_list_page_url},
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

    let session: Uuid = match args.session {
        Some(s) => {
            let uuid = Uuid::try_parse(&s)
                .context("Failed to parse UUID")?;
            match uuid.get_version() {
                Some(uuid::Version::Random) => uuid,
                _ => panic!("Only UUID v4 is allowed"),
            }
        },
        None => Uuid::new_v4()
    };

    // let update_assets = args.update_assets.unwrap_or(false);

    let app = App {
        pool: sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy(c.database_url.as_ref())
            .context("Failed to connect to postgres.")?,
    };

    let mut maybe_next_page_url = Some(c.list_page_url);
    while let Some(list_page_url) = maybe_next_page_url {
        let (_list_page, list_page_document) =
            save_list_page_url(&app.pool, &session, &list_page_url)
                .await
                .context("Failed to save list page from URL.")?;
        maybe_next_page_url = get_list_next_page_url(&list_page_document);
    }
    Ok(())
}
