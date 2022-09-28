use anyhow::Context;
use scraper::Html;

pub enum PageUrl {
    Storia(String),
    Olx(String),
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

pub fn get_list_next_page_url(document: &Html) -> Option<String> {
    // TODO: get rid of unwrap
    let selector = scraper::Selector::parse("a[data-cy=\"pagination-forward\"]").unwrap();
    document
        .select(&selector)
        .find_map(|item| item.value().attr("href").map(|href| format!("https://www.olx.ro{}", href)))
}

pub fn get_list_urls(document: &Html) -> Vec<PageUrl> {
    let selector =
        scraper::Selector::parse("div[data-testid=\"listing-grid\"] > div[data-cy=\"l-card\"] > a")
            .unwrap();
    document
        .select(&selector)
        .flat_map(|item| item.value().attr("href"))
        .flat_map(|url| PageUrl::parse(url).ok())
        .collect::<Vec<_>>()
}

pub async fn update_test_assets(list_page: &str) -> anyhow::Result<()> {
    let c = [("grid-list-page.html", list_page)];
    for (file_name, url) in c {
        download_page(url, file_name).await?;
    }
    Ok(())
}

pub async fn get_page(url: &str) -> anyhow::Result<String> {
    reqwest::Client::new()
        .get(url)
        .send()
        .await
        .context("Failed to request page.")?
        .error_for_status()
        .context("Response was not 200.")?
        .text()
        .await
        .context("Failed to parse the body as text")
}

pub async fn download_page(url: &str, file_name: &str) -> anyhow::Result<()> {
    let body = get_page(url).await?;

    tokio::fs::write(format!("tests/assets/{}", file_name), body)
        .await
        .context("Failed to write file.")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::utils::get_list_urls;

    #[tokio::test]
    async fn can_find_list_items() {
        let bytes = tokio::fs::read("./tests/assets/grid-list-page.html")
            .await
            .unwrap();
        let html = std::str::from_utf8(bytes.as_ref()).unwrap();

        let document = scraper::Html::parse_document(html);

        let results = get_list_urls(&document).len();

        // TODO: this is a crappy test, but will suffice for now
        // There are usually 45 items, but some might be ads or idk
        assert!(results >= 40);
    }
}
