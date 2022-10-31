use anyhow::Context;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub enum Currency {
    EUR,
    RON,
    USD,
}

impl TryFrom<&str> for Currency {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_lowercase().as_str() {
            "eur" => Ok(Self::EUR),
            "usd" => Ok(Self::USD),
            "ron" => Ok(Self::RON),
            _ => Err(anyhow::anyhow!("Cannot parse unknown currency."))
        }
    }
}

pub type PgTransaction<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

pub fn try_parse_session(s: &str) -> anyhow::Result<Uuid> {
    let session = Uuid::try_parse(s).context("Failed to parse UUID")?;
    if session
        .get_version()
        .map_or(true, |v| v != uuid::Version::Random)
    {
        return Err(anyhow::anyhow!("Only UUID v4 is allowed"));
    };
    Ok(session)
}
