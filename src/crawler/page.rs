use anyhow::Context;
use scraper::Html;
use url::Url;

use crate::{
    config::TEST_ASSETS_DIR,
    page::{PageType, PageUrl, SavedPage},
    util::PgTransaction,
};

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
        ) VALUES (CURRENT_TIMESTAMP, $1, $2, $3, $4)
        ON CONFLICT (session, url) DO NOTHING
        "#,
        &page.session,
        &page.url.to_string(),
        &page.page_type as &PageType,
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
    use super::get_list_urls;
    use crate::config::TEST_ASSETS_DIR;

    #[test]
    fn can_find_list_items() {
        let bytes = std::fs::read(format!("{}/grid-list-page.html", TEST_ASSETS_DIR)).unwrap();
        let html = std::str::from_utf8(bytes.as_ref()).unwrap();

        let document = scraper::Html::parse_document(html);

        let results = get_list_urls(&document).len();

        // TODO: this is a crappy test, but will suffice for now
        // There are usually 45 items, but some might be ads or idk
        assert!(results >= 38);
    }
}
