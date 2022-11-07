use anyhow::Context;

use crate::{config::Config, util::try_parse_session};

use super::extractor::{ExtractOptions, extract};

#[derive(clap::Args)]
pub struct ExtractCmd {
    pub session: String,
}

impl ExtractCmd {
    pub fn work(&self, config: &Config) -> anyhow::Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(async move {
                tracing_subscriber::fmt::init();
                tracing_appender::rolling::never("logs", "extracter.log");
                let session = try_parse_session(&self.session)?;

                let options = ExtractOptions {
                    config,
                    session,
                    pool: sqlx::postgres::PgPoolOptions::new()
                        .acquire_timeout(std::time::Duration::from_secs(2))
                        .connect_lazy(config.database_url.as_ref())
                        .context("Failed to establish lazy connection to postgres.")?,
                };

                extract(&options).await
            })
    }
}
