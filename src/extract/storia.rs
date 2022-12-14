use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};

use crate::util::Currency;

use super::{
    classified::{CardinalDirection, Classified, Layout, PropertyType, SellerType},
    extractor::SavedPage,
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Info {
    label: String,
    values: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Characteristic {
    key: String,
    value: String,
    label: String,
    localized_value: String,
    currency: String,
}

// #[derive(serde::Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct Image {
//     thumbnail: String,
//     small: String,
//     medium: String,
//     large: String,
// }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Owner {
    name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Agency {}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Breadcrumb {}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoriaClassified {
    created_at: DateTime<Utc>,
    title: String,
    top_information: Vec<Info>,
    characteristics: Vec<Characteristic>,
    // images: Vec<Image>,
    owner: Owner,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoriaClassifiedInnerWrapper {
    ad: StoriaClassified,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoriaClassifiedMiddleWrapper {
    page_props: StoriaClassifiedInnerWrapper,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoriaClassifiedOuterWrapper {
    props: StoriaClassifiedMiddleWrapper,
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

    let o = wrapper.props.page_props.ad;

    let (raw_price, raw_currency): (&str, &str) = o
        .characteristics
        .iter()
        .find_map(|c| {
            if c.key == "price" {
                return Some((c.value.as_str(), c.currency.as_str()));
            }
            None
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to find price characteristic"))?;

    Ok(Classified {
        session,
        url: &page.url,
        orientation: o
            .characteristics
            .iter()
            .find_map(|i| {
                if i.label == "main_solar_orient" {
                    return Some(i.value.as_str().trim());
                }
                None
            })
            .map(|o| CardinalDirection::try_from(o).map(Some))
            .unwrap_or(Ok(None))?,
        floor: o.top_information.iter().find_map(|p| {
            if p.label == "floor" {
                if let Some(f) = p.values.iter().find_map(|v| v.strip_prefix('/')) {
                    return f.parse::<i16>().ok();
                }
            }
            None
        }),
        layout: o.characteristics.iter().find_map(|c| {
            if c.key == "divisioning_type" {
                return Layout::try_from(c.localized_value.as_str()).ok();
            }
            None
        }),
        negotiable: false,
        price: raw_price.parse().context("Failed to parse price.")?,
        currency: Currency::try_from(raw_currency)?,
        property_type: o
            .characteristics
            .iter()
            .find_map(|c| {
                if c.key == "building_type" {
                    return Some(c.localized_value.as_str());
                }
                None
            })
            .ok_or_else(|| anyhow!("Failed to find property type."))
            .map(PropertyType::try_from)??,
        published_at: o.created_at,
        room_count: o
            .characteristics
            .iter()
            .find_map(|c| {
                if c.key == "rooms_num" {
                    return Some(
                        c.value
                            .parse::<i16>()
                            .map(Some)
                            .context("Failed to parse room count."),
                    );
                }
                None
            })
            .unwrap_or(Ok(None))?,
        seller_name: o.owner.name,
        seller_type: SellerType::Private,
        surface: None,
        title: o.title,
        year: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{super::extractor::SavedPage, extract_page_json, parse_classified};
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
