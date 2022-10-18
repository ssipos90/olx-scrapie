use anyhow::Context;
use sqlx::PgPool;

use crate::config::Config;

pub mod command;

pub struct ExtractOptions<'a> {
    pub config: &'a Config,
    pub pool: PgPool,
    pub session: uuid::Uuid,
}

pub async fn extract<'a>(options: &'a ExtractOptions<'a>) -> anyhow::Result<()> {
    if sqlx::query!(
        r#"
        SELECT session
        FROM sessions
        WHERE session=$1
        "#,
        options.session,
    )
    .fetch_optional(&options.pool)
    .await
    .context("Failed retrieving existing session.")?
    .is_none()
    {
        return Err(anyhow::anyhow!("No session found in DB."));
    }



    Ok(())
}
