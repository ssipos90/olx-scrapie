use anyhow::Context;
use chrono::Utc;
use scraper::Html;
use sqlx::PgPool;
use url::Url;

use crate::config::TEST_ASSETS_DIR;

#[derive(sqlx::FromRow)]
pub struct Page<'a> {
    pub crawled_at: chrono::DateTime<Utc>,
    pub session: &'a uuid::Uuid,
    pub url: String,
    pub page_type: PageType,
    pub content: String,
}

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "page_type", rename_all = "snake_case")]
pub enum PageType {
    OlxList,
    OlxItem,
    StoriaItem,
}

pub enum PageUrl {
    Storia(String),
    Olx(String),
}

impl TryInto<Url> for PageUrl {
    type Error = url::ParseError;
    fn try_into(self) -> Result<Url, Self::Error> {
        match self {
            PageUrl::Storia(url) => Url::parse(&url),
            PageUrl::Olx(url) => Url::parse(&url),
        }
    }
}

impl PageUrl {
    pub fn parse(url: &str) -> anyhow::Result<Self> {
        if url.starts_with("https://www.storia.ro") || url.starts_with("https://storia.ro") {
            return Ok(Self::Storia(url.to_string()));
        }

        if url.starts_with("/d/") {
            return Ok(Self::Olx(format!("https://www.olx.ro{}", url)));
        }

        Err(anyhow::anyhow!("Don't know how to handle {}", url))
    }
}

pub fn get_list_next_page_url(document: &Html) -> Option<Url> {
    // TODO: get rid of unwrap
    let selector = scraper::Selector::parse("div.pager a[data-cy=\"page-link-next\"]").unwrap();
    document
        .select(&selector)
        .find_map(|item| {
            item.value()
                .attr("href")
                .map(|href| if href.starts_with("https://") { href.to_string() } else { format!("https://www.olx.ro{}", href) })
        })
        .and_then(|url| Url::parse(&url).ok())
}

pub fn get_list_urls(document: &Html) -> Vec<PageUrl> {
    // let selector =
    //     scraper::Selector::parse("div[data-testid=\"listing-grid\"] > div[data-cy=\"l-card\"] > a")
    //         .unwrap();
    let selector = scraper::Selector::parse(r#"table#offers_table td.offer a[data-cy="listing-ad-title"]"#).unwrap();
    document
        .select(&selector)
        .flat_map(|item| item.value().attr("href"))
        .flat_map(|url| PageUrl::parse(url).ok())
        .collect::<Vec<_>>()
}

pub async fn get_page(url: &Url) -> anyhow::Result<String> {
    reqwest::Client::new()
        .get(url.to_string())
        .header("accept", "*/*")
        .header("user-agent", "curl/7.85.0")
        .send()
        .await
        .context("Failed to request page.")?
        .error_for_status()
        .context("Response was not 200.")?
        .text()
        .await
        .context("Failed to parse the body as text")
}

#[tracing::instrument(skip_all)]
pub async fn save_list_page_url<'a>(
    pool: &PgPool,
    session: &'a uuid::Uuid,
    list_page_url: &Url,
) -> anyhow::Result<(Page<'a>, Html)> {
    let list_page = Page {
        session,
        url: list_page_url.to_string(),
        page_type: PageType::OlxList,
        crawled_at: chrono::Utc::now(),
        content: get_page(list_page_url).await?,
    };
    tracing::info!("URL: {}", list_page.url);

    let list_page_document = scraper::Html::parse_document(&list_page.content);

    save_page(pool, &list_page)
        .await
        .context("Failed to save list page")?;

    let list_page_items = get_list_urls(&list_page_document);
    tracing::info!("found {} items.", list_page_items.len());

    for item_page_url in list_page_items {
        let item_page_url: Url = item_page_url.try_into()?;
        if item_page_url.domain().unwrap().contains("olx.ro/") {
            save_item_page_url(pool, session, &item_page_url).await?;
        }
    }

    Ok((list_page, list_page_document))
}

#[tracing::instrument(skip_all)]
async fn save_item_page_url<'a>(
    pool: &PgPool,
    session: &'a uuid::Uuid,
    item_page_url: &Url,
) -> anyhow::Result<Page<'a>> {
    let item_page = Page {
        session,
        url: item_page_url.to_string(),
        page_type: PageType::OlxList,
        crawled_at: chrono::Utc::now(),
        content: get_page(item_page_url).await?,
    };
    tracing::info!("URL: {}", item_page.url);
    save_page(pool, &item_page)
        .await
        .context("Failed to save item page")?;
    Ok(item_page)
}

#[tracing::instrument(skip_all)]
pub async fn save_page<'a>(pool: &PgPool, page: &Page<'a>) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO pages
        (
          crawled_at,
          session,
          url,
          page_type,
          content
        ) VALUES (NOW(), $1, $2, $3, $4)
        ON CONFLICT (session, url) DO NOTHING
        "#,
        &page.session,
        &page.url.to_string(),
        page.page_type as PageType,
        &page.content
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub fn save_test_asset(asset_name: &str, body: &str) -> anyhow::Result<()> {
    std::fs::write(format!("{}/{}", TEST_ASSETS_DIR, asset_name), body)
        .context("Failed to write test asset file.")
}

#[cfg(test)]
mod tests {
    use crate::{config::TEST_ASSETS_DIR, utils::get_list_urls};

    #[test]
    fn can_find_list_items() {
        let bytes = std::fs::read(format!("{}/grid-list-page.html", TEST_ASSETS_DIR)).unwrap();
        let html = std::str::from_utf8(bytes.as_ref()).unwrap();

        let document = scraper::Html::parse_document(html);

        let results = get_list_urls(&document).len();

        // TODO: this is a crappy test, but will suffice for now
        // There are usually 45 items, but some might be ads or idk
        assert!(results >= 40);
    }
}