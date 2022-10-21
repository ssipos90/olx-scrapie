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
    pub url: &'a str,

    pub face: &'b CardinalDirection,
    pub floor: u8,
    pub layout: Option<&'b Layout>,
    pub negotiable: bool,
    pub price: f32,
    pub property_type: &'b PropertyType,
    pub published_at: &'b str,
    pub room_count: u8,
    pub seller_name: &'b str,
    pub seller_type: &'b SellerType,
    pub surface: u32,
    pub title: &'b str,
    pub year: u32,
}
