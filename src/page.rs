use chrono::Utc;
use url::Url;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct SavedPage<'a> {
    pub content: String,
    pub crawled_at: chrono::DateTime<Utc>,
    pub page_type: PageType,
    pub session: &'a Uuid,
    pub url: String,
}

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "page_type", rename_all = "snake_case")]
pub enum PageType {
    OlxList,
    OlxItem,
    StoriaItem,
}

impl std::fmt::Display for PageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::OlxList => "OLX List",
                Self::OlxItem => "Olx Item",
                Self::StoriaItem => "Storia Item",
            }
        )
    }
}

pub enum PageUrl {
    StoriaItem(Url),
    OlxItem(Url),
}

impl std::fmt::Debug for PageUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StoriaItem(item) => f.debug_tuple("StoriaItem").field(item).finish(),
            Self::OlxItem(item) => f.debug_tuple("OlxItem").field(item).finish(),
        }
    }
}

impl From<PageUrl> for String {
    fn from(val: PageUrl) -> Self {
        match val {
            PageUrl::StoriaItem(url) => url,
            PageUrl::OlxItem(url) => url,
        }
        .to_string()
    }
}

impl From<&PageUrl> for PageType {
    fn from(val: &PageUrl) -> Self {
        match val {
            PageUrl::StoriaItem(_) => PageType::StoriaItem,
            PageUrl::OlxItem(_) => PageType::OlxItem,
        }
    }
}

impl AsRef<Url> for PageUrl {
    fn as_ref(&self) -> &Url {
        match self {
            Self::StoriaItem(url) => url,
            Self::OlxItem(url) => url,
        }
    }
}

impl PageUrl {
    pub fn parse(url: &str) -> anyhow::Result<Self> {
        if url.starts_with("https://www.storia.ro") || url.starts_with("https://storia.ro") {
            return Ok(Self::StoriaItem(Url::parse(url)?));
        }

        if url.starts_with("https://www.olx.ro/d/oferta/") {
            return Ok(Self::OlxItem(Url::parse(url)?));
        }

        if url.starts_with("/d/oferta/") {
            return Ok(Self::OlxItem(Url::parse(&format!(
                "https://www.olx.ro{}",
                url
            ))?));
        }

        Err(anyhow::anyhow!("Don't know how to handle {}", url))
    }
}
