use chrono::{DateTime, Utc};

use crate::util::Currency;

use super::extractor::SavedPage;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OlxClassifiedLocation {
    city_name: String,
    region_name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OlxClassifiedParam {
    key: String,
    name: String,
    normalized_value: String,
    value: String,
}

#[derive(serde::Deserialize)]
struct OlxClassifiedUser {
    company_name: String,
    created: DateTime<Utc>,
    name: String,
    #[serde(rename = "camelCase")]
    seller_type: Option<()>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OlxClassifiedRegularPrice {
    value: f32,
    currency_code: Currency,
    negotiable: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OlxClassifiedPrice {
    regular_price: OlxClassifiedRegularPrice,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OlxClassified {
    created_time: DateTime<Utc>,
    description: String,
    is_active: bool,
    is_business: bool,
    is_highlighted: bool,
    is_promoted: bool,
    last_refresh_time: DateTime<Utc>,
    location: OlxClassifiedLocation,
    params: Vec<OlxClassifiedParam>,
    // https://frankfurt.apollo.olxcdn.com:443/v1/files/2i2w3927ow9i3-RO/image;s=429x537"
    photos: Vec<String>,
    price: OlxClassifiedPrice,
    title: String,
    status: String,
    user: OlxClassifiedUser,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OlxClassifiedInnerWrapper {
    ad: OlxClassified,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OlxClassifiedWrapper {
    ad: OlxClassifiedInnerWrapper,
}

const JS_JSON_LINE_PREFIX: &str = "        window.__PRERENDERED_STATE__= ";

pub fn extract_page_json(page: &SavedPage) -> anyhow::Result<String> {
    let selector = scraper::Selector::parse("script#olx-init-config").unwrap();
    let document = scraper::Html::parse_document(&page.content);

    let el = document
        .select(&selector)
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to find script element in page."))?;

    let text_it = el
        .text()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to extract script element text node."))?;

    let script: &str = text_it
        .lines()
        .find_map(|line| {
            line.strip_prefix(JS_JSON_LINE_PREFIX).map(|s| {
                if s.len() > 4 {
                    Ok(&s[1..s.len() - 2])
                } else {
                    Err(anyhow::anyhow!("Javascript JSON string is too short."))
                }
            })
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to find begining of string pattern."))??;

    unescape::unescape(script)
        .ok_or_else(|| anyhow::anyhow!("Failed to unescape Javascript JSON string."))
}

#[cfg(test)]
mod tests {
    use super::{super::extractor::SavedPage, extract_page_json, OlxClassifiedWrapper};
    use crate::page::PageType;

    #[test]
    fn expected_json_fields() {
        let page = SavedPage {
            url: "https://www.olx.ro/d/oferta/garsoniera-uzina-2-IDgC0Kq.html".into(),
            content: String::from_utf8(
                std::fs::read("src/extract/test_assets/extract_olx_works.html").unwrap(),
            )
            .unwrap(),
            page_type: PageType::OlxItem,
            crawled_at: chrono::offset::Utc::now(),
        };
        let json = extract_page_json(&page).unwrap();

        let _olx_classified: OlxClassifiedWrapper = serde_json::from_str(json.as_str()).unwrap();
    }
}
