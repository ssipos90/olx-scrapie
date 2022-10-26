use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use scraper::{Html, Selector};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Config, page::PageType, session::Session};

use super::classified::{CardinalDirection, Classified, Layout, PropertyType, SellerType};

pub struct SavedPage {
    pub content: String,
    pub crawled_at: DateTime<Utc>,
    pub page_type: PageType,
    pub url: String,
}

pub struct ExtractOptions<'a> {
    pub config: &'a Config,
    pub pool: PgPool,
    pub session: uuid::Uuid,
}

pub struct ExtractorSelectors {
    published_at: Selector,
    title: Selector,
    price: Selector,
    negotiable: Selector,
    seller_type: Selector, // person or agency
    seller_name: Selector,
    layout: Selector, // decomandat, etc
    surface: Selector,
    face: Selector, // north, south, etc
    year: Selector,
    floor: Selector,         // TODO: include demisol
    property_type: Selector, // Apartment, house, etc
    room_count: Selector,
}

struct Extractor {
    olx: ExtractorSelectors,
    // storia: ExtractorSelectors,
}

fn parse_selector(selector: &str) -> anyhow::Result<scraper::Selector> {
    scraper::Selector::parse(selector)
        .map_err(|e| anyhow::anyhow!("Failed to parse selector {:?}", e))
}

impl Extractor {
    fn find_room_number(s: &str) -> Option<i16> {
        if s.contains("1 camera") || s.contains("garsoniera") || s.contains("Garsoniera") {
            return Some(1);
        }
        if s.contains("2 camere") || s.contains("2 Camere") {
            return Some(2);
        }
        if s.contains("3 camere") || s.contains("3 Camere") {
            return Some(3);
        }
        if s.contains("camere") {
            return Some(4);
        }
        None
    }

    fn try_new() -> anyhow::Result<Self> {
        Ok(Self {
            olx: ExtractorSelectors {
                published_at: parse_selector(r#"span[data-cy="ad-posted-at"]"#)?,
                title: parse_selector(r#"h1[data-cy="ad_title"]"#)?,
                price: parse_selector(r#"div[data-testid="ad-price-container"] h3"#)?,
                negotiable: parse_selector(
                    r#"div[data-testid="ad-price-container"] p[data-testid=negotiable-label]"#,
                )?,
                seller_type: parse_selector(
                    r#"div[data-cy="seller_card"] div[data-testid="trader-title"]"#,
                )?,
                seller_name: parse_selector(
                    r#"div[data-cy="seller_card"] a[data-testid="user-profile-link"] h4"#,
                )?,
                layout: parse_selector(r#"div[data-testid="qa-advert-slot"]+ul ul li"#)?,
                surface: parse_selector(r#"div[data-testid="qa-advert-slot"]+ul ul li"#)?,
                face: parse_selector(r#"div[data-cy="ad_description"]"#)?,
                year: parse_selector(r#"div[data-testid="qa-advert-slot"]+ul ul li"#)?,
                floor: parse_selector(r#"div[data-testid="qa-advert-slot"]+ul ul li"#)?,
                property_type: parse_selector(r#"div[data-cy="ad_description"] > div"#)?,
                room_count: parse_selector(
                    r#"h1[data-cy="title"], div[data-cy="ad_description"] > div"#,
                )?,
            },
            // storia: ExtractorSelectors {
            // },
        })
    }

    fn extract<'a, 'b>(
        &self,
        session: &'a Uuid,
        page: &'b SavedPage,
    ) -> anyhow::Result<Classified<'a, 'b>> {
        let document = Html::parse_document(&page.content);
        let smf = document
            .select(&self.olx.published_at)
            .find_map(|item| {
                let s = item.text().fold(String::new(), |a, b| a + b).trim();
                if s.len() == 0 {
                    None
                } else {
                    Some(s)
                }
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to find published_at."))
            .map(|s| {
                if let Some(s) = s.strip_prefix("Postat azi la ") {
                    let time = chrono::NaiveTime::parse_from_str(s, "%H:%M")
                        .context("Failed matching today's time in published at.")?;
                    return chrono::offset::Utc::today()
                        .and_time(time)
                        .context("Failed to update published at time.");
                }

                if let Some(s) = s.strip_prefix("Postat la ") {
                    return DateTime::parse_from_str(s, "%d %b %Y")
                        .context("Failed to parse published_at")
                        .map(|date| date.with_timezone(&Utc));
                }
                Err(anyhow::anyhow!("Nothing matched published at patterns."))
            })??;
        match page.page_type {
            PageType::OlxItem => Ok(Classified {
                session,
                url: &page.url,
                published_at: smf,
                title: document
                    .select(&self.olx.title)
                    .find_map(|item| item.text().map(|s| s.to_string()).next())
                    .ok_or_else(|| anyhow::anyhow!("Failed to find title."))?,
                price: document
                    .select(&self.olx.price)
                    .find_map(|item| item.text().find(|_| true))
                    .ok_or_else(|| anyhow::anyhow!("Failed to find price."))?
                    .parse()?,
                negotiable: document
                    .select(&self.olx.negotiable)
                    .find_map(|item| item.text().find(|_| true))
                    .map_or(false, |s| s.contains("negociabil")),
                seller_type: document
                    .select(&self.olx.seller_type)
                    .find_map(|item| item.text().find(|_| true))
                    .map_or(Ok(SellerType::Company), SellerType::try_from)?,
                seller_name: document
                    .select(&self.olx.seller_name)
                    .find_map(|item| item.text().map(|s| s.to_string()).next())
                    .ok_or_else(|| anyhow::anyhow!("Failed to find seller name."))?,
                layout: document
                    .select(&self.olx.layout)
                    .find_map(|item| item.text().find_map(Layout::find_in_str)),
                surface: document
                    .select(&self.olx.surface)
                    .find_map(|item| item.text().find_map(|s| s.parse().ok())),
                property_type: document
                    .select(&self.olx.property_type)
                    .find_map(|item| item.text().find_map(PropertyType::find_in_str))
                    .unwrap_or(PropertyType::Apartment),
                face: document
                    .select(&self.olx.face)
                    .find_map(|item| item.text().find_map(CardinalDirection::find_in_str)),
                year: document
                    .select(&self.olx.year)
                    .find_map(|item| item.text().find_map(|s| s.parse().ok())),
                floor: document
                    .select(&self.olx.floor)
                    .find_map(|item| item.text().find_map(|s| s.parse().ok())),
                room_count: document
                    .select(&self.olx.room_count)
                    .find_map(|item| item.text().find_map(Self::find_room_number)),
            }),
            // PageType::StoriaItem => self.storia,
            _ => Err(anyhow::anyhow!("Only item pages have extractors.")),
        }
    }
}

pub async fn extract<'a>(options: &'a ExtractOptions<'a>) -> anyhow::Result<()> {
    let session = load_session(&options.pool, &options.session)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No session found in DB."))?;

    tracing::info!("Session loaded from database ({:?}).", session.crawled_at);

    if session.crawled_at.is_none() {
        return Err(anyhow::anyhow!(
            "Session did not finish crawling, currently not supported."
        ));
    }

    let workers = futures::stream::iter((0..4).into_iter().map(|c| {
        tracing::info!("Spawned worker {}.", c);
        tokio::spawn(work(options.pool.clone(), session.session))
    }))
    .buffer_unordered(3)
    .collect::<Vec<_>>();

    workers.await;

    Ok(())
}

async fn work(pool: PgPool, session: Uuid) -> anyhow::Result<()> {
    let sleepy = std::time::Duration::from_secs(1);

    let extractor = match Extractor::try_new() {
        Ok(ex) => ex,
        Err(e) => {
            tracing::error!("Failed to initialize extractor ({:?})", e);
            return Err(anyhow::anyhow!(e));
        }
    };

    loop {
        match load_saved_page(&pool, &session).await {
            Ok(Some(page)) => {
                tracing::info!("Extracting {}", &page.url);
                let classified = extractor.extract(&session, &page)?;

                sqlx::query!(
                    r#"
                    INSERT INTO classifieds
                    (
                        session,
                        url,
                        revision,
                        extracted_at,

                        face,
                        floor,
                        layout,
                        negotiable,
                        price,
                        property_type,
                        published_at,
                        room_count,
                        seller_name,
                        seller_type,
                        surface,
                        title,
                        year
                    )
                    VALUES (
                        $1,
                        $2,
                        1,
                        CURRENT_TIMESTAMP,

                        $3,
                        $4,
                        $5,
                        $6,
                        $7,
                        $8,
                        $9,
                        $10,
                        $11,
                        $12,
                        $13,
                        $14,
                        $15
                    );
                    "#,
                    &session,
                    &classified.url,
                    classified.face as Option<CardinalDirection>,
                    classified.floor,
                    classified.layout as Option<Layout>,
                    &classified.negotiable,
                    classified.price,
                    classified.property_type as PropertyType,
                    &classified.published_at,
                    classified.room_count,
                    &classified.seller_name,
                    classified.seller_type as SellerType,
                    classified.surface,
                    &classified.title,
                    classified.year
                )
                .execute(&pool)
                .await?;
            }
            Ok(None) => {
                tracing::info!("No more pages to extract, breaking...");
                break;
            }
            Err(sqlx::Error::PoolTimedOut) => {
                tracing::warn!("Pool timed out, pausing a bit...");
                std::thread::sleep(sleepy);
            }
            Err(e) => {
                tracing::error!("Failed to retrieve page ({:?})", e);
                return Err(anyhow::anyhow!(e));
            }
        };
    }
    tracing::info!("Finished working.");

    Ok(())
}

async fn load_session(pool: &PgPool, session: &Uuid) -> anyhow::Result<Option<Session>> {
    sqlx::query_as!(
        Session,
        r#"
        SELECT
          session,
          created_at,
          crawled_at
        FROM sessions
        WHERE session=$1
        "#,
        session,
    )
    .fetch_optional(pool)
    .await
    .context("Failed retrieving session.")
}

async fn load_saved_page(pool: &PgPool, session: &Uuid) -> Result<Option<SavedPage>, sqlx::Error> {
    sqlx::query_as!(
        SavedPage,
        r#"
        SELECT
            p.content,
            p.crawled_at,
            p.page_type as "page_type: _",
            p.url
        FROM pages AS p
        WHERE session=$1
        AND page_type IN ('olx_item', 'storia_item')
            AND NOT EXISTS (
                SELECT session
                FROM classifieds AS c
                WHERE c.session=p.session
                    AND c.url=p.url
            )
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#,
        session
    )
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::{Extractor, SavedPage};
    use crate::page::PageType;

    #[test]
    fn initialize_extractor() {
        assert!(Extractor::try_new().is_ok());
    }

    #[test]
    fn extract_olx_works() {
        let extractor = Extractor::try_new().unwrap();
        let session = uuid::Uuid::new_v4();
        let page = SavedPage {
            url: "https://www.olx.ro/d/oferta/garsoniera-uzina-2-IDgC0Kq.html".into(),
            content: String::from_utf8(
                std::fs::read("src/extract/test_assets/extract_olx_works.html").unwrap(),
            )
            .unwrap(),
            page_type: PageType::OlxItem,
            crawled_at: chrono::offset::Utc::now(),
        };
        let classified = extractor.extract(&session, &page).unwrap();

        assert_eq!(classified.title, "garsoniera Uzina 2");
    }
}
