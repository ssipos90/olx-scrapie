use anyhow::Context;
use clap::Parser;
use olx_scrapie::{
    config::Config,
    utils::{get_list_next_page_url, get_list_urls, get_page, save_test_asset},
};
use sqlx::{types::time::Date, PgPool};
use uuid::Uuid;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(clap::Parser)]
struct Cli {
    update_assets: Option<bool>,
    session: String,
}

struct App {
    pool: PgPool,
}

#[derive(sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
enum PageType {
    OlxList,
    OlxItem,
    StoriaItem,
}

#[derive(sqlx::FromRow)]
struct Page {
    added_at: Date,
    session: Uuid,
    url: String,
    page_type: PageType,
    content: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let c = Config::from_env().context("Failed to create the configuration.")?;

    let args = Cli::parse();

    let session = Uuid::try_parse(&args.session).context("Fialed to parse session UUID")?;

    // let update_assets = args.update_assets.unwrap_or(false);

    let app = App {
        pool: sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy(&c.database_url.to_string())
            .context("Failed to connect to postgres.")?,
    };

    let mut maybe_next_page_url = Some(c.list_page_url);
    while let Some(next_page_url) = maybe_next_page_url {
        let list_page_content = get_page(&next_page_url)?;

        sqlx::query!(
            r#"
            INSERT INTO pages
            (
              added_at,
              session,
              url,
              page_type,
              content
            ) VALUES (
              NOW(),
              $1,
              $2,
              $3,
              $4
            )
            "#,
            &session,
            next_page_url.to_string(),
            PageType::OlxList,
            &list_page_content
        )
        .execute(&app.pool)
        .await
        .context("Failed to insert page")?;

        let list_page_document = scraper::Html::parse_document(&list_page_content);

        let list_page_items = get_list_urls(&list_page_document);
        println!(
            "found {} items on url {}",
            list_page_items.len(),
            next_page_url
        );
        for item_url in list_page_items {
            let page_content = get_page(&item_url.try_into()?)?;
        }
        maybe_next_page_url = get_list_next_page_url(&list_page_document);
    }
    Ok(())
}
