use crate::page::{
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

impl std::fmt::Display for RetrievedCrawlJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", &self.page_type, &self.url)
    }
}

pub async fn process_jobs(pool: &PgPool, session: &uuid::Uuid) {
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
                  AND session=$1
                FOR UPDATE
                SKIP LOCKED
                LIMIT 1
                "#,
                session
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
                    }
                },
                Ok(None) => {
                    tracing::info!("No more immediate jobs in queue");
                    transaction.rollback().await.ok();
                    match sqlx::query!(
                        r#"
                        SELECT
                          not_before
                        FROM crawler_queue
                        WHERE
                          status IN ('new', 'retrying')
                          AND session=$1
                        LIMIT 1
                        "#,
                        session
                    )
                    .fetch_optional(pool)
                    .await
                    {
                        Ok(Some(result)) => {
                            let utc: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
                            let timeout = result.not_before.signed_duration_since(utc);
                            tracing::info!(
                                "Next job in {} seconds, sleeping...",
                                timeout.num_seconds()
                            );

                            std::thread::sleep(timeout.to_std().unwrap());
                        }
                        Ok(None) => {
                            tracing::info!("No defered jobs in queue. Exiting.");
                            return;
                        }
                        Err(e) => {
                            tracing::error!("Failed to check for defered job {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to fetch next job {:?}", e);
                }
            };
        }
    }
}

#[tracing::instrument(skip_all)]
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

#[tracing::instrument(skip_all, fields(job = %job))]
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
            let current_retries = job.retries.len() + 1;
            if current_retries >= MAX_RETRIES {
                tracing::info!(
                    "Exhausted MAX_RETRIES, marking as failed with error: {:?}",
                    e
                );
                mark_failed(transaction, job, &e).await?;
            } else {
                tracing::error!("Failed with retryable error: {:?}.", e);
                tracing::info!("Re-queueing job (count: {}).", current_retries);
                sqlx::query!(
                    r#"
                    UPDATE crawler_queue
                    SET
                      status='failed',
                      retries=array_append(retries, $3),
                      not_before=CURRENT_TIMESTAMP
                    WHERE session=$1
                    AND url=$2
                    "#,
                    &job.session,
                    &job.url,
                    e.to_string(),
                )
                .execute(transaction)
                .await?;
            }
        }
        Err(ProcessedJobError::FatalError(e)) => {
            tracing::error!("Failed with fatal error: {:?}.", e);
            mark_failed(transaction, job, &e).await?;
        }
    };
    Ok(())
}

enum ProcessedJobError {
    RetryableError(anyhow::Error),
    FatalError(anyhow::Error),
}

#[tracing::instrument(skip_all)]
async fn run_job<'a>(
    transaction: &mut PgTransaction<'a>,
    job: &RetrievedCrawlJob,
) -> Result<(), ProcessedJobError> {
    let url = url::Url::parse(&job.url)
        .context("Failed to parse error.")
        .map_err(ProcessedJobError::FatalError)?;

    match job.page_type {
        PageType::OlxItem => {
            tracing::info!("saving olx item page: {}", &url);
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
            tracing::info!("saving storia item page: {}", &url);
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
            tracing::info!("saving olx list page: {}", &url);
            let content = get_page(&url)
                .await
                .map_err(ProcessedJobError::RetryableError)?;
            let document = scraper::Html::parse_document(&content);
            if let Some(url) = get_list_next_page_url(&document) {
                tracing::info!("Found next page url");
                insert_job(transaction, &job.session, &url, PageType::OlxList)
                    .await
                    .map_err(ProcessedJobError::RetryableError)?;
            }
            let pages_urls = get_list_urls(&document);
            tracing::info!("Found {} item urls", pages_urls.len());
            for page_url in pages_urls {
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
