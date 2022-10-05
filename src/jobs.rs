use sqlx::PgPool;

use crate::utils::{get_page, save_item_page_url, PageType};

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "crawl_status", rename_all = "snake_case")]
pub enum CrawlStatus {
    New,
    Retrying,
    Success,
    Failed,
}

#[derive(sqlx::FromRow)]
pub struct CrawlJob {
    status: CrawlStatus,
    session: uuid::Uuid,
    url: String,
    page_type: PageType,
}

pub async fn process_job(pool: &PgPool) -> Result<(), anyhow::Error> {
    loop {
        let mut transaction = pool.begin().await?;
        let job_result = sqlx::query_as!(
            CrawlJob,
            r#"
            SELECT
              status as "status: _",
              session,
              url,
              page_type as "page_type: _"
            FROM crawler_queue
            LIMIT 1
            FOR UPDATE
            "#
        )
        .fetch_optional(&mut transaction)
        .await;
        match job_result {
            Ok(Some(job)) => {
                let url = url::Url::parse(&job.url);
                match job.page_type {
                    PageType::OlxItem => {
                        save_item_page_url(&mut transaction, &job.session, &url)
                    }
                    PageType::OlxList => todo!(),
                    PageType::StoriaItem => todo!(),
                };
                sqlx::query!(
                    r#"
                    UPDATE crawler_queue
                    SET status='completed'
                    WHERE session=$1
                    AND url=$2
                    "#,
                    job.session,
                    job.url
                )
                .execute(&mut transaction)
                .await;
            }
            Ok(None) => todo!(),
            Err(e) => todo!(),
        };
    }
}
