use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "seller_type", rename_all = "snake_case")]
pub enum SellerType {
    Private,
    Company,
}

impl TryFrom<&str> for SellerType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "privat" => Ok(Self::Private),
            "firma" => Ok(Self::Company),
            _ => Err(anyhow::anyhow!("Failed to parse seller type.")),
        }
    }
}

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "proparty_layout", rename_all = "snake_case")]
pub enum Layout {
    Wagon,
    SemiFancy,
    Fancy,
}

impl TryFrom<&str> for Layout {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "vagon" => Ok(Self::Wagon),
            "semidecomandat" => Ok(Self::SemiFancy),
            "decomandat" => Ok(Self::Fancy),
            _ => Err(anyhow::anyhow!("Failed to parse layout.")),
        }
    }
}

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "cardinal_direction", rename_all = "snake_case")]
pub enum CardinalDirection {
    North,
    South,
    East,
    West,
}

impl TryFrom<&str> for CardinalDirection {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "nord" | "nordic" => Ok(Self::North),
            "sud" | "sudic" => Ok(Self::South),
            "est" | "estic" => Ok(Self::East),
            "vest" | "vestic" => Ok(Self::West),
            _ => Err(anyhow::anyhow!("Failed to parse facing direction.")),
        }
    }
}

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "property_type", rename_all = "snake_case")]
pub enum PropertyType {
    Apartment,
    House,
}

const APARTMENT_MATCHES: [&str;6] = [
    "apartament",
    "Apartament",
    "garsoniera",
    "Garsoniera",
    "studio",
    "Studio",
];

const HOUSE_MATCHES: [&str;4] = [
    "casa",
    "Casa",
    "vila",
    "Vila",
];

impl PropertyType {
    fn from_str (default: Self, value: &str) -> Self {
        if APARTMENT_MATCHES.iter().any(|&m| value.contains(m)) {
            return Self::Apartment;
        }
        if HOUSE_MATCHES.iter().any(|&m| value.contains(m)) {
            return Self::House;
        }

        default
    }
}

impl TryFrom<&str> for PropertyType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "apartament" => Ok(Self::Apartment),
            "casa" => Ok(Self::House),
            _ => Err(anyhow::anyhow!("Failed to parse property type.")),
        }
    }
}

pub struct Classified<'a, 'b> {
    pub session: &'a Uuid,
    pub url: &'b str,

    pub face: Option<CardinalDirection>,
    pub floor: Option<i16>,
    pub layout: Option<Layout>,
    pub negotiable: bool,
    pub price: f64,
    pub property_type: PropertyType,
    pub published_at: DateTime<Utc>,
    pub room_count: Option<i16>,
    pub seller_name: String,
    pub seller_type: SellerType,
    pub surface: Option<i32>,
    pub title: String,
    pub year: Option<i32>,
}
