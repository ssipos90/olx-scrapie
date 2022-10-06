use anyhow::Context;
use chrono::Utc;
use scraper::Html;
use url::Url;

use crate::config::TEST_ASSETS_DIR;

pub type PgTransaction<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

#[derive(sqlx::FromRow)]
pub struct SavedPage<'a> {
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

impl std::fmt::Display for PageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::OlxList => "OLX List",
                Self::OlxItem => "Olx Item",
                Self::StoriaItem => "Storia Item",
            }
        )
    }
}

pub enum PageUrl {
    StoriaItem(Url),
    OlxItem(Url),
}

impl From<PageUrl> for String {
    fn from(val: PageUrl) -> Self {
        match val {
            PageUrl::StoriaItem(url) => url,
            PageUrl::OlxItem(url) => url,
        }
        .to_string()
    }
}

impl From<&PageUrl> for PageType {
    fn from(val: &PageUrl) -> Self {
        match val {
            PageUrl::StoriaItem(_) => PageType::StoriaItem,
            PageUrl::OlxItem(_) => PageType::OlxItem,
        }
    }
}

impl AsRef<Url> for PageUrl {
    fn as_ref(&self) -> &Url {
        match self {
            Self::StoriaItem(url) => url,
            Self::OlxItem(url) => url,
        }
    }
}

impl PageUrl {
    pub fn parse(url: &str) -> anyhow::Result<Self> {
        if url.starts_with("https://www.storia.ro") || url.starts_with("https://storia.ro") {
            return Ok(Self::StoriaItem(Url::parse(url)?));
        }

        if url.starts_with("https://www.olx.ro/d/oferta/") {
            return Ok(Self::OlxItem(Url::parse(url)?));
        }

        if url.starts_with("/d/oferta/") {
            return Ok(Self::OlxItem(Url::parse(&format!(
                "https://www.olx.ro{}",
                url
            ))?));
        }

        Err(anyhow::anyhow!("Don't know how to handle {}", url))
    }
}

pub fn get_list_next_page_url(document: &Html) -> Option<Url> {
    let selector = scraper::Selector::parse("div.pager a[data-cy=\"page-link-next\"]").unwrap();
    document
        .select(&selector)
        .find_map(|item| {
            item.value().attr("href").map(|href| {
                if href.starts_with("https://") {
                    href.to_string()
                } else {
                    format!("https://www.olx.ro{}", href)
                }
            })
        })
        .and_then(|url| Url::parse(&url).ok())
}

pub fn get_list_urls(document: &Html) -> Vec<PageUrl> {
    let selector =
        scraper::Selector::parse(r#"table#offers_table td.offer a[data-cy="listing-ad-title"]"#)
            .unwrap();
    document
        .select(&selector)
        .flat_map(|item| item.value().attr("href"))
        .flat_map(|url| PageUrl::parse(url).ok())
        .collect::<Vec<_>>()
}

#[tracing::instrument(skip_all, fields(url = %url))]
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

#[tracing::instrument(skip_all, fields(url = %url))]
pub async fn save_list_page_url<'a>(
    transaction: &mut PgTransaction<'a>,
    session: &'a uuid::Uuid,
    url: &Url,
) -> anyhow::Result<(SavedPage<'a>, Html)> {
    let list_page = SavedPage {
        session,
        url: url.to_string(),
        page_type: PageType::OlxList,
        crawled_at: chrono::Utc::now(),
        content: get_page(url).await?,
    };
    tracing::info!("URL: {}", list_page.url);

    let list_page_document = scraper::Html::parse_document(&list_page.content);

    save_page(transaction, &list_page)
        .await
        .context("Failed to save list page")?;

    let list_page_items = get_list_urls(&list_page_document);
    tracing::info!("found {} items.", list_page_items.len());

    // for item_page_url in list_page_items {
    //     save_item_page_url(pool, session, &item_page_url).await?;
    // }

    Ok((list_page, list_page_document))
}

#[tracing::instrument(skip_all, fields(url = %url))]
pub async fn save_item_page<'a, 'b>(
    transaction: &mut PgTransaction<'a>,
    session: &'b uuid::Uuid,
    url: &Url,
) -> anyhow::Result<SavedPage<'b>> {
    let item_page = SavedPage {
        session,
        url: url.to_string(),
        page_type: PageType::OlxItem,
        crawled_at: chrono::Utc::now(),
        content: get_page(url).await?,
    };
    tracing::info!("URL: {}", item_page.url);
    save_page(transaction, &item_page)
        .await
        .context("Failed to save item page")?;
    Ok(item_page)
}

#[tracing::instrument(skip_all)]
pub async fn save_page<'a, 'b>(
    transaction: &mut PgTransaction<'a>,
    page: &SavedPage<'b>,
) -> sqlx::Result<()> {
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
    .execute(transaction)
    .await?;
    Ok(())
}

pub fn save_test_asset(asset_name: &str, body: &str) -> anyhow::Result<()> {
    std::fs::write(format!("{}/{}", TEST_ASSETS_DIR, asset_name), body)
        .context("Failed to write test asset file.")
}

#[cfg(test)]
mod tests {
    use crate::{config::TEST_ASSETS_DIR, page::get_list_urls};

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
