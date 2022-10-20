use anyhow::Context;
use clap::Parser;
use olx_scrapie::{
    config::Config,
    crawler::command::CrawlCmd,
    extract::command::ExtractCmd,
    list_sessions::ListSessionsCmd,
};

/// Search for a pattern in a file and display the lines that contain it.
#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    ListSessions(ListSessionsCmd),
    Crawl(CrawlCmd),
    Extract(ExtractCmd),
}

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cfg = Config::from_env().context("Failed to load the env configuration.")?;

    let args = Cli::parse();

    // let update_assets = args.update_assets.unwrap_or(false);

    match args.command {
        Commands::ListSessions(cmd) => cmd.work(&cfg),
        Commands::Crawl(cmd) => cmd.work(&cfg),
        Commands::Extract(cmd) => cmd.work(&cfg),
    }
}
