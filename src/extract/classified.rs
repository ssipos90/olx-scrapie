use uuid::Uuid;

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

pub enum PropertyType {
    Apartment,
    House,
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
    pub floor: Option<u8>,
    pub layout: Option<Layout>,
    pub negotiable: bool,
    pub price: f32,
    pub property_type: PropertyType,
    pub published_at: String,
    pub room_count: Option<u8>,
    pub seller_name: String,
    pub seller_type: SellerType,
    pub surface: Option<u32>,
    pub title: String,
    pub year: Option<u32>,
}
