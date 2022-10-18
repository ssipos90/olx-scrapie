pub mod command;
pub mod job;
pub mod page;

use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;

use self::{
    job::{insert_job, process_jobs},
    page::PageType,
};

pub struct CrawlOptions<'a> {
    pub config: &'a Config,
    pub pool: PgPool,
    pub session: Option<uuid::Uuid>,
}

#[tracing::instrument(skip_all)]
pub async fn crawl<'a>(options: &'a CrawlOptions<'a>) -> anyhow::Result<()> {
    let session = match options.session {
        Some(session) => {
            tracing::info!("Reusing session {}", session);

            if sqlx::query!(
                r#"
                SELECT session
                FROM sessions
                WHERE session=$1
                "#,
                session,
            )
            .fetch_optional(&options.pool)
            .await
            .context("Failed retrieving existing session.")?
            .is_none()
            {
                return Err(anyhow::anyhow!("No session found in DB."));
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
            SET completed_at=CURRENT_TIMESTAMP
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