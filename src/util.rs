use anyhow::Context;
use uuid::Uuid;

pub fn try_parse_session(s: &str) -> anyhow::Result<Uuid> {
    let session = Uuid::try_parse(s)
        .context("Failed to parse UUID")?;
    if session
        .get_version()
        .map_or(true, |v| v != uuid::Version::Random)
    {
        return Err(anyhow::anyhow!("Only UUID v4 is allowed"));
    };
    Ok(session)
}
