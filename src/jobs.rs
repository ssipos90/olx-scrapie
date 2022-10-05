use crate::utils::{
    get_list_next_page_url, get_list_urls, get_page, save_page, PageType, PgTransaction, SavedPage,
};
use anyhow::Context;
use sqlx::PgPool;

const MAX_RETRIES: usize = 3;

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "crawl_status", rename_all = "snake_case")]
pub enum CrawlStatus {
    New,
    Retrying,
    Success,
    Failed,
}

#[derive(sqlx::FromRow)]
pub struct RetrievedCrawlJob {
    session: uuid::Uuid,
    url: String,
    page_type: PageType,
    retries: Vec<String>,
}

pub async fn process_jobs(pool: &PgPool) {
    loop {
        if let Ok(mut transaction) = pool.begin().await {
            let job_result = sqlx::query_as!(
                RetrievedCrawlJob,
                r#"
                SELECT
                  session,
                  url,
                  page_type as "page_type: _",
                  retries
                FROM crawler_queue
                WHERE
                  status IN ('new', 'retrying')
                  AND not_before >= NOW()
                LIMIT 1
                FOR UPDATE
                "#
            )
            .fetch_optional(&mut transaction)
            .await;
            match job_result {
                Ok(Some(job)) => match process_job(&mut transaction, &job).await {
                    Ok(_) => {
                        transaction.commit().await.ok();
                    }
                    Err(e) => {
                        tracing::error!("Failed to process job {:?}", e);
                        return;
                    },
                },
                Ok(None) => {
                    tracing::info!("No more jobs in queue");
                    return;
                }
                Err(e) => {
                    tracing::error!("Failed to fetch next job {:?}", e);
                }
            };
        }
    }
}

async fn mark_failed<'a>(
    transaction: &mut PgTransaction<'a>,
    job: &RetrievedCrawlJob,
    e: &anyhow::Error,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE crawler_queue
        SET
            status='failed',
            failure_error=$1
        WHERE session=$2
        AND url=$3
        "#,
        e.to_string(),
        &job.session,
        &job.url,
    )
    .execute(transaction)
    .await?;
    Ok(())
}

async fn process_job<'a>(
    transaction: &mut PgTransaction<'a>,
    job: &RetrievedCrawlJob,
) -> anyhow::Result<()> {
    match run_job(transaction, job).await {
        Ok(_) => {
            sqlx::query!(
                r#"
                UPDATE crawler_queue
                SET status='completed'
                WHERE session=$1
                AND url=$2
                "#,
                &job.session,
                &job.url
            )
            .execute(transaction)
            .await
            .context("Failed to update crawled job status")?;
        }
        Err(ProcessedJobError::RetryableError(e)) => {
            if job.retries.len() >= MAX_RETRIES - 1 {
                mark_failed(transaction, job, &e).await?;
            } else {
                sqlx::query!(
                    r#"
                    UPDATE crawler_queue
                    SET
                      status='failed',
                      retries=array_append(retries, $3),
                      not_before=CURRENT_TIMESTAMP + ($4 * interval '1 minute')
                    WHERE session=$1
                    AND url=$2
                    "#,
                    &job.session,
                    &job.url,
                    e.to_string(),
                    1.0
                )
                .execute(transaction)
                .await?;
            }
        }
        Err(ProcessedJobError::FatalError(e)) => {
            mark_failed(transaction, job, &e).await?;
        }
    };
    Ok(())
}

enum ProcessedJobError {
    RetryableError(anyhow::Error),
    FatalError(anyhow::Error),
}

async fn run_job<'a>(
    transaction: &mut PgTransaction<'a>,
    job: &RetrievedCrawlJob,
) -> Result<(), ProcessedJobError> {
    let url = url::Url::parse(&job.url)
        .context("Failed to parse error.")
        .map_err(ProcessedJobError::FatalError)?;
    match job.page_type {
        PageType::OlxItem => {
            let page = SavedPage {
                session: &job.session,
                url: url.to_string(),
                page_type: job.page_type,
                crawled_at: chrono::Utc::now(),
                content: get_page(&url)
                    .await
                    .map_err(ProcessedJobError::RetryableError)?,
            };
            save_page(transaction, &page)
                .await
                .context("Failed to save page")
                .map_err(ProcessedJobError::RetryableError)?;
        }
        PageType::StoriaItem => {
            let page = SavedPage {
                session: &job.session,
                url: url.to_string(),
                page_type: job.page_type,
                crawled_at: chrono::Utc::now(),
                content: get_page(&url)
                    .await
                    .map_err(ProcessedJobError::RetryableError)?,
            };
            save_page(transaction, &page)
                .await
                .context("Failed to save page")
                .map_err(ProcessedJobError::RetryableError)?;
        }
        PageType::OlxList => {
            let content = get_page(&url)
                .await
                .map_err(ProcessedJobError::RetryableError)?;
            let document = scraper::Html::parse_document(&content);
            if let Some(url) = get_list_next_page_url(&document) {
                insert_job(transaction, &job.session, &url, PageType::OlxList)
                    .await
                    .map_err(ProcessedJobError::RetryableError)?;
            }
            for page_url in get_list_urls(&document) {
                insert_job(
                    transaction,
                    &job.session,
                    page_url.as_ref(),
                    PageType::from(&page_url),
                )
                .await
                .map_err(ProcessedJobError::RetryableError)?;
            }
        }
    };

    Ok(())
}

pub async fn insert_job<'a>(
    transaction: &mut PgTransaction<'a>,
    session: &uuid::Uuid,
    url: &url::Url,
    page_type: PageType,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO crawler_queue (
            session,
            url,
            page_type,
            added_at,
            not_before
        ) VALUES (
            $1, $2, $3, NOW(), NOW()
        )
        ON CONFLICT (session, url) DO NOTHING
        "#,
        session,
        url.as_str(),
        page_type as PageType,
    )
    .execute(transaction)
    .await
    .context("Failed to insert into cralwer_queue.")
    .map(|_| ())
}
