mod utils;

use anyhow::Context;
use utils::{get_page, get_list_urls, get_list_next_page_url, update_test_assets};

use clap::Parser;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(clap::Parser)]
struct Cli {
    /// The pattern to look for
    #[clap(subcommand)]
    command: CliCommand,
}

#[derive(clap::Subcommand)]
enum CliCommand {
    UpdateTestAssets,
    BuildItemList,
}

const LIST_PAGE: &str = 
        "https://www.olx.ro/d/imobiliare/apartamente-garsoniere-de-inchiriat/2-camere/brasov/?search[order]=created_at:desc";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        CliCommand::UpdateTestAssets => {
            update_test_assets(LIST_PAGE)
                .await
                .context("Failed to update test assets.")?;
        },
        CliCommand::BuildItemList => {
            let mut maybe_next_page_url = Some(LIST_PAGE.to_string());
            while let Some(next_page_url) = maybe_next_page_url {
                let list_page_content = get_page(&next_page_url)
                    .await?;

                let list_page_document = scraper::Html::parse_document(&list_page_content);

                let list_page_items = get_list_urls(&list_page_document);
                println!("found {} items on url {}", list_page_items.len(), next_page_url);
                maybe_next_page_url = get_list_next_page_url(&list_page_document);
            }
        },
    }
    Ok(())
}
