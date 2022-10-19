use chrono::{DateTime, Utc};
use futures::StreamExt;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Config, crawler::page::SavedPage};

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

async fn work(_pool: &PgPool) -> anyhow::Result<()> {
    todo!()
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
            .map(|_| tokio::spawn(work(&options.pool.clone()))),
    )
    .buffer_unordered(3);

    let results = sqlx::query(
        r#"
        SELECT
            p.*
        FROM pages AS p
        WHERE session=?
            AND NOT EXISTS (
                SELECT session
                FROM classifieds AS c
                WHERE c.session=p.session
                    AND c.url=p.url
            )
        "#,
    )
    .bind(&session.session)
    .fetch(&options.pool);

    while let Some(page) = results.try_into().await.context("Mno")? {
        rx.try_recv()
    }

    Ok(())
}

pub async fn load_session(pool: &PgPool, session: &Uuid) -> anyhow::Result<Option<Session>> {
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
