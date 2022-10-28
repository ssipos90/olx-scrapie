use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};

// use crate::util::Currency;

use super::{
    classified::{Classified, PropertyType, SellerType},
    extractor::SavedPage,
};

// #[derive(serde::Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct OlxClassifiedLocation {
//     city_name: String,
//     region_name: String,
// }

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct OlxClassifiedParam {
    key: String,
    // name: String,
    // normalized_value: String,
    value: String,
}

#[derive(serde::Deserialize)]
struct OlxClassifiedUser {
    company_name: String,
    // created: DateTime<Utc>,
    name: String,
    // #[serde(rename = "camelCase")]
    // seller_type: Option<()>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OlxClassifiedRegularPrice {
    value: f64,
    // currency_code: Currency,
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
    // is_active: bool,
    // is_business: bool,
    // is_highlighted: bool,
    // is_promoted: bool,
    // last_refresh_time: DateTime<Utc>,
    // location: OlxClassifiedLocation,
    params: Vec<OlxClassifiedParam>,
    // https://frankfurt.apollo.olxcdn.com:443/v1/files/2i2w3927ow9i3-RO/image;s=429x537"
    // photos: Vec<String>,
    price: OlxClassifiedPrice,
    title: String,
    // status: todo!(),
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
    let selector = scraper::Selector::parse("script#olx-init-config")
        .map_err(|e| anyhow!("Failed to parse selector {:?}", e))?;
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

pub fn parse_classified<'a, 'b>(
    session: &'a uuid::Uuid,
    page: &'b SavedPage,
) -> anyhow::Result<Classified<'a, 'b>> {
    let json = extract_page_json(page).context("Failed extracting OLX JSON from page.")?;

    let olx_classified_wrapper: OlxClassifiedWrapper =
        serde_json::from_str(json.as_str()).context("Failed parsing OLX JSON.")?;

    let o = olx_classified_wrapper.ad.ad;

    Ok(Classified {
        session,
        url: &page.url,
        face: o.params.iter().find_map(|_| None),
        floor: o
            .params
            .iter()
            .find_map(|p| match p.key == "floor" {
                true => Some(p.value.as_str()),
                _ => None,
            })
            .map_or(Ok(None), |v| match v {
                "parter" | "Parter" => Ok(Some(0)),
                v => v.parse().map(Some).context("Failed parsing OLX floor"),
            })?,
        layout: o.params.iter().find_map(|_| None),
        negotiable: o.price.regular_price.negotiable,
        price: o.price.regular_price.value,
        property_type: PropertyType::find_in_str(o.description.as_str())
            .unwrap_or(PropertyType::Apartment),
        published_at: o.created_time,
        room_count: None, // TODO:
        seller_name: o.user.name,
        seller_type: match o.user.company_name.is_empty() {
            true => SellerType::Private,
            _ => SellerType::Company,
        },
        surface: None,
        title: o.title,
        year: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        super::extractor::SavedPage, extract_page_json, parse_classified, OlxClassifiedWrapper,
    };
    use crate::page::PageType;

    #[test]
    fn expected_json_fields() {
        let page = SavedPage {
            url: "https://www.olx.ro/d/oferta/garsoniera-uzina-2-IDgC0Kq.html".into(),
            content: String::from_utf8(
                std::fs::read("src/extract/test_assets/olx-item.html").unwrap(),
            )
            .unwrap(),
            page_type: PageType::OlxItem,
            crawled_at: chrono::offset::Utc::now(),
        };
        let json = extract_page_json(&page).unwrap();

        let _olx_classified: OlxClassifiedWrapper = serde_json::from_str(json.as_str()).unwrap();
    }

    #[test]
    fn parse_classifieds() {
        // TODO: maybe loop over multiple
        let session = uuid::Uuid::new_v4();
        let page = SavedPage {
            url: "https://www.olx.ro/d/oferta/garsoniera-uzina-2-IDgC0Kq.html".into(),
            content: String::from_utf8(
                std::fs::read("src/extract/test_assets/olx-item.html").unwrap(),
            )
            .unwrap(),
            page_type: PageType::OlxItem,
            crawled_at: chrono::offset::Utc::now(),
        };

        if let Err(e) = parse_classified(&session, &page) {
            panic!("{:?}", e);
        }
    }
}
