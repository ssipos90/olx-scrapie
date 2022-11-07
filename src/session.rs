use anyhow::Context;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::config::Config;

#[derive(clap::Args)]
pub struct ListSessionsCmd {}

impl ListSessionsCmd {
    pub fn work(&self, config: &Config) -> anyhow::Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(async move {
                tracing_appender::rolling::never("logs", "session.log");
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .connect(config.database_url.as_ref())
                    .await
                    .context("Failed to establish connection to postgres.")?;

                sqlx::query!(
                    r#"
                    SELECT
                    *
                    FROM sessions;
                    "#
                )
                .fetch_all(&pool)
                .await
                .context("Failed to load sessions from database.")?
                .iter()
                .for_each(|session| {
                    println!(
                        "{} | {} | {}",
                        session.session,
                        session.created_at,
                        session
                            .crawled_at
                            .map_or("-".into(), |crawled_at| crawled_at.to_string())
                    );
                });

                Ok(())
            })
    }
}

pub struct Session {
    pub crawled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub session: Uuid,
}
