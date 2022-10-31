use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::util::Currency;

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

impl Layout {
    pub fn find_in_str(s: &str) -> Option<Self> {
        if s.contains("vagon") {
            return Some(Self::Wagon);
        }
        if s.contains("semidecomandat") {
            return Some(Self::SemiFancy);
        }
        if s.contains("decomandat") {
            return Some(Self::Fancy);
        }
        None
    }
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

impl CardinalDirection {
    pub fn find_in_str(s: &str) -> Option<Self> {
        let s = s.to_lowercase();
        if s.contains("la sud") || s.contains("sudica") {
            return Some(Self::South);
        }
        if s.contains("la est") || s.contains("estica") {
            return Some(Self::East);
        }
        if s.contains("la vest") || s.contains("vestica") {
            return Some(Self::West);
        }
        if s.contains("la nord") || s.contains("nordica") {
            return Some(Self::North);
        }
        None
    }
}

impl TryFrom<&str> for CardinalDirection {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "north" | "nord" | "nordic" => Ok(Self::North),
            "south" | "sud" | "sudic" => Ok(Self::South),
            "east" | "est" | "estic" => Ok(Self::East),
            "west" | "vest" | "vestic" => Ok(Self::West),
            _ => Err(anyhow::anyhow!("Failed to parse cardinal direction.")),
        }
    }
}

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "property_type", rename_all = "snake_case")]
pub enum PropertyType {
    Apartment,
    House,
}

const APARTMENT_MATCHES: [&str; 6] = [
    "apartament",
    "Apartament",
    "garsoniera",
    "Garsoniera",
    "studio",
    "Studio",
];

const HOUSE_MATCHES: [&str; 4] = ["casa", "Casa", "vila", "Vila"];

impl PropertyType {
    pub fn find_in_str(value: &str) -> Option<Self> {
        if APARTMENT_MATCHES.iter().any(|&m| value.contains(m)) {
            return Some(Self::Apartment);
        }
        if HOUSE_MATCHES.iter().any(|&m| value.contains(m)) {
            return Some(Self::House);
        }

        None
    }
}

impl TryFrom<&str> for PropertyType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "apartament" | "garsoniera" => Ok(Self::Apartment),
            "casa" => Ok(Self::House),
            _ => Err(anyhow::anyhow!("Failed to parse property type.")),
        }
    }
}

pub struct Classified<'a, 'b> {
    pub session: &'a Uuid,
    pub url: &'b str,

    pub orientation: Option<CardinalDirection>,
    pub floor: Option<i16>,
    pub layout: Option<Layout>,
    pub negotiable: bool,
    pub price: f64,
    pub currency: Currency,
    pub property_type: PropertyType,
    pub published_at: DateTime<Utc>,
    pub room_count: Option<i16>,
    pub seller_name: String,
    pub seller_type: SellerType,
    pub surface: Option<i32>,
    pub title: String,
    pub year: Option<i32>,
}
