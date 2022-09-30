use anyhow::Context;
use scraper::Html;
use url::Url;

use crate::config::TEST_ASSETS_DIR;

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
    let selector = scraper::Selector::parse("a[data-cy=\"pagination-forward\"]").unwrap();
    document
        .select(&selector)
        .find_map(|item| {
            item.value()
                .attr("href")
                .map(|href| format!("https://www.olx.ro{}", href))
        })
        .and_then(|url| Url::parse(&url).ok())
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

pub fn get_page(url: &Url) -> anyhow::Result<String> {
    reqwest::blocking::Client::new()
        .get(url.to_string())
        .send()
        .context("Failed to request page.")?
        .error_for_status()
        .context("Response was not 200.")?
        .text()
        .context("Failed to parse the body as text")
}

pub fn save_test_asset(asset_name: &str, body: &str) -> anyhow::Result<()> {
    std::fs::write(format!("{}/{}", TEST_ASSETS_DIR, asset_name), body)
        .context("Failed to write test asset file.")
}

#[cfg(test)]
mod tests {
    use crate::{utils::get_list_urls, config::TEST_ASSETS_DIR};

    #[test]
    fn can_find_list_items() {
        let bytes = std::fs::read(format!("{}/grid-list-page.html", TEST_ASSETS_DIR))
            .unwrap();
        let html = std::str::from_utf8(bytes.as_ref()).unwrap();

        let document = scraper::Html::parse_document(html);

        let results = get_list_urls(&document).len();

        // TODO: this is a crappy test, but will suffice for now
        // There are usually 45 items, but some might be ads or idk
        assert!(results >= 40);
    }
}
