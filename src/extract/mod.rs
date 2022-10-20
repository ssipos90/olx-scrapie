use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Config, crawler::page::PageType};

pub mod command;

pub struct ExtractOptions<'a> {
    pub config: &'a Config,
    pub pool: PgPool,
    pub session: uuid::Uuid,
}

pub struct Session {
    pub session: uuid::Uuid,
    pub created_at: DateTime<Utc>,
    pub crawled_at: Option<DateTime<Utc>>,
}

pub struct SavedPage {
    pub crawled_at: chrono::DateTime<Utc>,
    pub url: String,
    pub content: String,
    pub page_type: PageType,
}

pub struct Classified<'a, 'b> {
    pub session: &'a uuid::Uuid,
    pub url: &'b String,
    pub title: String,
    pub a: String,
    pub b: String,
    pub c: String,
}

impl<'a, 'b> Classified<'a, 'b> {
    fn from_olx_item(session: &'a Uuid, page: &'b SavedPage) -> anyhow::Result<Self> {
        Ok(Self {
            session,
            url: &page.url,
            title: "".into(),
            a: "".into(),
            b: "".into(),
            c: "".into(),
        })
    }

    fn from_storia_item(session: &'a Uuid, page: &'b SavedPage) -> anyhow::Result<Self> {
        Ok(Self {
            session,
            url: &page.url,
            title: "".into(),
            a: "".into(),
            b: "".into(),
            c: "".into(),
        })
    }
}

async fn work(pool: PgPool, session: Uuid) -> anyhow::Result<()> {
    let sleepy = std::time::Duration::from_secs(1);

    loop {
        match load_saved_page(&pool, &session).await {
            Ok(Some(page)) => {
                let classified = match page.page_type {
                    PageType::OlxItem => Classified::from_olx_item(&session, &page)?,
                    PageType::StoriaItem => Classified::from_storia_item(&session, &page)?,
                    _ => return Err(anyhow::anyhow!("Only item pages can be extracted.")),
                };
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
