use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Config, extract::olx, extract::storia, page::PageType, session::Session};

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
        tokio::spawn(spawn_worker(options.pool.clone(), session.session))
    }))
    .buffer_unordered(3)
    .collect::<Vec<_>>();

    workers.await;

    Ok(())
}

async fn spawn_worker(pool: PgPool, session: Uuid) -> anyhow::Result<()> {
    let sleepy = std::time::Duration::from_secs(1);

    loop {
        match load_saved_page(&pool, &session).await {
            Ok(Some(page)) => {
                tracing::info!("Extracting {}", &page.url);
                let classified = match page.page_type {
                    PageType::OlxList => todo!(),
                    PageType::OlxItem => olx::parse_classified(&session, &page)?,
                    PageType::StoriaItem => storia::parse_classified(&session, &page)?,
                };
                save_classified(&pool, &classified).await?;
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

async fn save_classified<'a, 'b>(
    pool: &PgPool,
    classified: &Classified<'a, 'b>,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO classifieds
        (
            session,
            url,
            revision,
            extracted_at,

            orientation,
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
        classified.session,
        &classified.url,
        classified.orientation as Option<CardinalDirection>,
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
    .execute(pool)
    .await
    .map(|_| ())
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
