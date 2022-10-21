use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use scraper::{Html, Selector};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Config, page::PageType, session::Session};

use super::classified::{Classified, SellerType, Layout};

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

impl Extractor {
    fn try_new() -> anyhow::Result<Self> {
        Ok(Self {
            olx: ExtractorSelectors {
                published_at: scraper::Selector::parse(r#"span[data-cy="ad-posted-at"]"#)?,
                title: scraper::Selector::parse(r#"h1[data-cy="ad_title"]"#)?,
                price: scraper::Selector::parse(r#"div[data-testid="ad-price-container"] h3"#)?,
                negotiable: scraper::Selector::parse(
                    r#"div[data-testid="ad-price-container"] p[data-testid=negotiable-label]"#,
                )?,
                seller_type: scraper::Selector::parse(
                    r#"div[data-cy="seller_card"] div[data-testid="trader-title"]"#,
                )?,
                seller_name: scraper::Selector::parse(
                    r#"div[data-cy="seller_card"] a[data-testid="user-profile-link"] h4"#,
                )?,
                layout: scraper::Selector::parse(r#"div[data-testid="qa-advert-slot"]+ul ul li"#)?,
                surface: scraper::Selector::parse(r#"div[data-testid="qa-advert-slot"]+ul ul li"#)?,
                face: scraper::Selector::parse(r#""#)?,
                year: scraper::Selector::parse(r#"div[data-testid="qa-advert-slot"]+ul ul li"#)?,
                floor: scraper::Selector::parse(r#"div[data-testid="qa-advert-slot"]+ul ul li"#)?,
                property_type: scraper::Selector::parse(r#""#)?,
                room_count: scraper::Selector::parse(r#""#)?,
            },
            // storia: ExtractorSelectors {
            // },
        })
    }

    fn extract(&self, page: &SavedPage) -> anyhow::Result<()> {
        let document = Html::parse_document(&page.content);

        let data = match page.page_type {
            PageType::OlxItem => Classified {
                session: page.session,
                url: &page.url,
                published_at: document
                    .select(&self.olx.published_at)
                    .find_map(|item| item.text().find(|_| true))
                    .ok_or_else(|| anyhow::anyhow!("Failed to find published at date."))?,
                title: document
                    .select(&self.olx.title)
                    .find_map(|item| item.text().find(|_| true))
                    .ok_or_else(|| anyhow::anyhow!("Failed to find title."))?,
                price: document
                    .select(&self.olx.title)
                    .find_map(|item| item.text().find(|_| true))
                    .ok_or_else(|| anyhow::anyhow!("Failed to find price."))?
                    .parse::<f32>()?,
                negotiable: document
                    .select(&self.olx.negotiable)
                    .find_map(|item| item.text().find(|_| true))
                    .map_or(false, |s| s.contains("negociabil")),
                seller_type: &document
                    .select(&self.olx.seller_type)
                    .find_map(|item| item.text().find(|_| true))
                    .map_or(Ok(SellerType::Company), SellerType::try_from)?,
                seller_name: document
                    .select(&self.olx.seller_name)
                    .find_map(|item| item.text().find(|_| true))
                    .ok_or_else(|| anyhow::anyhow!("Failed to find seller name."))?,
                layout: &document
                    .select(&self.olx.seller_type)
                    .find_map(|item| item.text().find(|_| true))
                    .map(|x|)
                    .map(|s| Layout::try_from(s)),
                surface: document
                    .select(&self.olx.surface)
                    .find_map(|item| item.text().find(|_| true)),
                property_type: document
                    .select(&self.olx.property_type)
                    .find_map(|item| item.text().find(|_| true)),
                face: document
                    .select(&self.olx.property_type)
                    .find_map(|item| item.text().find(|_| true)),
                year: document
                    .select(&self.olx.property_type)
                    .find_map(|item| item.text().find(|_| true)),
                floor: document
                    .select(&self.olx.property_type)
                    .find_map(|item| item.text().find(|_| true)),
                room_count: document
                    .select(&self.olx.property_type)
                    .find_map(|item| item.text().find(|_| true)),
            },
            // PageType::StoriaItem => self.storia,
            _ => return Err(anyhow::anyhow!("Only item pages have extractors.")),
        };

        Ok(())
    }
}

pub async fn extract<'a>(options: &'a ExtractOptions<'a>) -> anyhow::Result<()> {
    let session = load_session(&options.pool, &options.session)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No session found in DB."))?;

    if session.crawled_at.is_none() {
        return Err(anyhow::anyhow!(
            "Session did not finish crawling, currently not supported."
        ));
    }

    let workers = futures::stream::iter(
        (0..4)
            .into_iter()
            .map(|_| tokio::spawn(work(options.pool.clone(), session.session))),
    )
    .buffer_unordered(3)
    .collect::<Vec<_>>();

    workers.await;

    Ok(())
}

async fn work(pool: PgPool, session: Uuid) -> anyhow::Result<()> {
    let sleepy = std::time::Duration::from_secs(1);

    let extractor = Extractor::try_new()?;

    loop {
        match load_saved_page(&pool, &session).await {
            Ok(Some(page)) => {
                let classified = extractor.extract(&page)?;

                sqlx::query!(
                    r#"
                    INSERT INTO classifieds
                    (session, url, revision, extracted_at)
                    VALUES (
                        $1,
                        $2,
                        1,
                        CURRENT_TIMESTAMP
                    );
                    "#,
                    &session,
                    &classified.url
                )
                .execute(&pool)
                .await?;
            }
            Ok(None) => break,
            Err(sqlx::Error::PoolTimedOut) => {
                std::thread::sleep(sleepy);
            }
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
    }

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
