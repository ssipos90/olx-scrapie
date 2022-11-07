use anyhow::Context;
use uuid::Uuid;

use crate::{config::Config, util::try_parse_session};

use super::{crawl, CrawlOptions};

#[derive(clap::Args)]
pub struct CrawlCmd {
    pub session: Option<String>,
}

impl CrawlCmd {
    pub fn work(&self, config: &Config) -> anyhow::Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(async move {
                tracing_appender::rolling::never("logs", "crawler.log");

                let session: Option<Uuid> = match &self.session {
                    Some(s) => Some(try_parse_session(s)?),
                    None => None,
                };
                let options = CrawlOptions {
                    session,
                    config,
                    pool: sqlx::postgres::PgPoolOptions::new()
                        .acquire_timeout(std::time::Duration::from_secs(2))
                        .connect_lazy(config.database_url.as_ref())
                        .context("Failed to establish lazy connection to postgres.")?,
                };
                crawl(&options).await
            })
    }
}
