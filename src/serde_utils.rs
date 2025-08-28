use chrono::TimeZone;
use serde::Deserialize;

pub fn deserialize_opt_f32_string<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        match s.parse::<f32>().map_err(serde::de::Error::custom) {
            Ok(r) => Ok(Some(r)),
            Err(e) => Err(e),
        }
    }
}
pub fn deserialize_f32_string<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse::<f32>().map_err(serde::de::Error::custom)
}

pub fn serialize_date_time<S, Tz: TimeZone>(
    val: &chrono::DateTime<Tz>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i64(val.timestamp())
}
