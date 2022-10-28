use anyhow::{anyhow, Context};
use chrono::{DateTime, Offset, Utc};

use crate::util::Currency;

use super::{
    classified::{Classified, PropertyType, SellerType},
    extractor::SavedPage,
};

#[derive(serde::Deserialize)]
struct StoriaClassifiedOwner {
    name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Info {
    label: String,
    values: Vec<String>,
    unit: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Characteristic {
    key: String,
    value: String,
    label: String,
    localized_value: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Image {
    thumbnail: String,
    small: String,
    medium: String,
    large: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Owner {
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Agency {
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Breadcrumb {
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoriaClassified {
    advertizer_type: String,
    created_at: DateTime<Utc>,
    title: String,
    top_information: Vec<Info>,
    additional_information: Vec<Info>,
    characteristics: Vec<Characteristic>,
    images: Vec<Image>,
    owner: Owner,
    agency: Agency, // TODO: maybe get phone number
    breadcrumbs: Vec<Breadcrumb>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoriaClassifiedInnerWrapper {
    ad: StoriaClassified,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoriaClassifiedMiddleWrapper {
    page_props:StoriaClassifiedInnerWrapper,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoriaClassifiedOuterWrapper {
    page: StoriaClassifiedMiddleWrapper,
}

pub fn extract_page_json(page: &SavedPage) -> anyhow::Result<String> {
    let selector = scraper::Selector::parse("script#__NEXT_DATA__")
        .map_err(|e| anyhow!("Failed to parse selector {:?}", e))?;
    let document = scraper::Html::parse_document(&page.content);

    document
        .select(&selector)
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to find script element in page."))?
        .text()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to extract script element text node."))
        .map(|s| s.to_string())
}

pub fn parse_classified<'a, 'b>(
    session: &'a uuid::Uuid,
    page: &'b SavedPage,
) -> anyhow::Result<Classified<'a, 'b>> {
    let json = extract_page_json(page).context("Failed extracting Storia JSON from page.")?;

    let wrapper: StoriaClassifiedOuterWrapper =
        serde_json::from_str(json.as_str()).context("Failed parsing Storia JSON.")?;

    let o = wrapper.page.page_props.ad;

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
                v => v.parse().map(Some).context("Failed parsing Storia floor"),
            })?,
        layout: o.params.iter().find_map(|_| None),
        negotiable: o.price.regular_price.negotiable,
        price: o.price.regular_price.value,
        property_type: PropertyType::find_in_str(o.description.as_str())
            .unwrap_or(PropertyType::Apartment),
        published_at: o.created_time,
        room_count: None, // TODO:
        seller_name: o.user.name,
        seller_type: SellerType::Private,
        surface: None,
        title: o.title,
        year: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        super::extractor::SavedPage, extract_page_json, parse_classified,
    };
    use crate::page::PageType;

    const ITEMS: [(&str, &str); 1] = [(
        "https://www.storia.ro/ro/oferta/inchiriere-garsoniera-lux-urban-plaza-IDtVQ3.html",
        "src/extract/test_assets/storia-item.html",
    )];

    #[test]
    fn found_json() {
        let (url, file) = ITEMS[0];
        let page = SavedPage {
            url: url.into(),
            content: String::from_utf8(std::fs::read(file).unwrap()).unwrap(),
            page_type: PageType::StoriaItem,
            crawled_at: chrono::offset::Utc::now(),
        };

        if let Err(e) = extract_page_json(&page) {
            panic!("{:?}", e);
        }
    }

    #[test]
    fn parse_json() {
        let (url, file) = ITEMS[0];
        let session = uuid::Uuid::new_v4();
        let page = SavedPage {
            url: url.into(),
            content: String::from_utf8(std::fs::read(file).unwrap()).unwrap(),
            page_type: PageType::StoriaItem,
            crawled_at: chrono::offset::Utc::now(),
        };

        if let Err(e) = parse_classified(&session, &page) {
            panic!("{:?}", e);
        }
    }
}
