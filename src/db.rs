use sqlx::PgPool;

pub async fn init() -> anyhow::Result<PgPool> {
    PgPool::connect(&dotenvy::var("DATABASE_URL")?)
        .await
        .map_err(anyhow::Error::msg)
}

#[derive(Debug, Clone, Copy)]
pub enum QueryOrdering {
    ASC,
    DESC,
}

#[derive(Debug)]
pub struct DecodeQueryOrderingError;
impl std::fmt::Display for DecodeQueryOrderingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for DecodeQueryOrderingError {}

impl<'a> TryFrom<&'a str> for QueryOrdering {
    type Error = DecodeQueryOrderingError;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match &*(value.to_uppercase()) {
            "ASC" => Ok(QueryOrdering::ASC),
            "DESC" => Ok(QueryOrdering::DESC),
            _ => Err(DecodeQueryOrderingError),
        }
    }
}

impl<'de> serde::Deserialize<'de> for QueryOrdering {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = serde::Deserialize::deserialize(deserializer)?;
        (*s).try_into().map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for QueryOrdering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                QueryOrdering::ASC => "ASC",
                QueryOrdering::DESC => "DESC",
            }
        )
    }
}
