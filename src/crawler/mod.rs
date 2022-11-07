pub mod command;
pub mod job;
pub mod page;

use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Config, page::PageType};

use self::job::{insert_job, process_jobs};

pub struct CrawlOptions<'a> {
    pub config: &'a Config,
    pub pool: PgPool,
    pub session: Option<uuid::Uuid>,
}

pub async fn crawl<'a>(options: &'a CrawlOptions<'a>) -> anyhow::Result<()> {
    let session = match options.session {
        Some(session) => {
            tracing::info!("Reusing session {}", session);

            match sqlx::query!(
                r#"
                SELECT
                    session,
                    crawled_at
                FROM sessions
                WHERE session=$1
                "#,
                session,
            )
            .fetch_optional(&options.pool)
            .await
            .context("Failed retrieving existing session.")?
            {
                Some(s) => {
                    if s.crawled_at.is_some() {
                        return Err(anyhow::anyhow!("Session is already crawled."));
                    }
                },
                None => return Err(anyhow::anyhow!("No session found in DB.")),
            };

            session
        }
        None => {
            let session = Uuid::new_v4();
            tracing::info!("New session: {}", session);

            let mut transaction = options.pool.begin().await?;
            sqlx::query!(
                r#"
                INSERT INTO sessions
                (session, created_at)
                VALUES ($1, CURRENT_TIMESTAMP)
                "#,
                &session
            )
            .execute(&mut transaction)
            .await
            .context("Failed saving new session.")?;

            insert_job(
                &mut transaction,
                &session,
                &options.config.list_page_url,
                PageType::OlxList,
            )
            .await?;

            transaction.commit().await?;

            session
        }
    };

    if process_jobs(&options.pool, &session).await.is_ok() {
        let result = sqlx::query!(
            r#"
            UPDATE sessions
            SET crawled_at=CURRENT_TIMESTAMP
            WHERE session=$1
            "#,
            &session
        )
        .execute(&options.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("No session has been updated, lol wut?"));
        }
    }

    Ok(())
}
