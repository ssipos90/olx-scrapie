use anyhow::Context;
use std::env::var;

pub const TEST_ASSETS_DIR: &str = "tests/crawler/assets";

pub struct Config {
    pub database_url: url::Url,
    pub list_page_url: url::Url,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: var("DATABASE_URL")
                .context("DATABASE_URL missing, cannot parse")
                .and_then(|s| url::Url::parse(&s).context("Failed to parse DATABASE_URL env var."))?,
            list_page_url: var("LIST_PAGE_URL")
                .context("LIST_PAGE missing, cannot parse")
                .and_then(|s| url::Url::parse(&s).context("Failed to parse LIST_PAGE env var."))?,
        })
    }
}
